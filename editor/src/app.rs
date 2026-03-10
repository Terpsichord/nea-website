use crate::{
    buffer::{Buffer, BufferError, Buffers, FileData},
    color_scheme::AvailableColorSchemes,
    explorer::{Explorer, ExplorerAction},
    platform::{self, FileSystemTrait as _, RunnerTrait as _, SearchResult},
};

use core::f32;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use eframe::egui;
use egui::{
    Align, Button, CentralPanel, Color32, ComboBox, Grid, Id, Key, KeyboardShortcut, Layout, MenuBar, Modifiers, RichText, ScrollArea, SidePanel, Style, TopBottomPanel, ViewportCommand, Visuals, containers::modal::Modal
};
use egui_extras::syntax_highlighting;
#[cfg(not(target_arch = "wasm32"))]
use egui_term::{TerminalBackend, TerminalView};
use eyre::OptionExt;
use uuid::Uuid;
use ws_messages::{ColorScheme, EditorSettings};

#[macro_export]
macro_rules! dbg_frame {
    ($body:expr) => {
        |ui| dbg_frame!(ui, $body)
    };
    ($ui:expr, $body:expr) => {
        dbg_frame!($ui, egui::Color32::ORANGE, $body)
    };
    ($ui:expr, $color:expr, $body:expr) => {{
        egui::containers::Frame::new()
            .stroke((1.0, $color))
            .show($ui, $body)
            .inner
    }};
}

#[derive(PartialEq, Debug)]
pub enum ModalAction {
    OpenFile,
    OpenFolder,
    DeleteBuffer(Uuid),
    Close,
}

#[derive(Debug, Clone, Copy)]
enum SaveError {
    NoFileSelected,
    NoBufferSelected,
}

/// The current state of the save modal
#[derive(Default, Debug)]
enum SaveModalState {
    /// Open (saves all files on "Save")
    SaveAllOpen,
    /// Open (saves current file on "Save")
    SaveFileOpen,
    /// In the process of closing
    ///
    /// Needed to visably close the modal before performing the modal action
    Closing,
    /// Closed (and the modal action is completed)
    #[default]
    Closed,
}

#[derive(Default, Debug)]
struct SearchModalState {
    search_text: String,
    replace_text: String,
    is_replace: bool,
    search_results: Vec<SearchResult>,
}

#[derive(Clone, Copy, Debug)]
enum BottomPanelState {
    Output,
    Terminal,
}

#[derive(Default)]
pub struct App {
    editor_settings: EditorSettings,
    project: Option<platform::Project>,
    fs: platform::FileSystem,
    runner: platform::Runner,
    buffers: Buffers,
    code_theme: syntax_highlighting::CodeTheme,
    style: Option<Style>,
    available_color_schemes: AvailableColorSchemes,
    /// [`Explorer`] side panel
    explorer: Option<Explorer>,
    bottom_panel_state: Option<BottomPanelState>,
    /// Contents of the output panel
    /// This must be wrapped in an `Arc<Mutex<_>>` so that it can be shared to and modified across threads, including the `running_command` thread.
    output: Arc<Mutex<String>>,
    #[cfg(not(target_arch = "wasm32"))]
    terminal: Option<TerminalBackend>,
    error_message: Option<String>,
    /// Current state of the save modal, as described in [`SaveModalState`]
    save_modal_state: SaveModalState,
    /// State of the settings modal
    settings_modal_state: Option<EditorSettings>,
    /// Current state of the modal for search and replace
    search_modal_state: Option<SearchModalState>,
    /// Whether the help modal is currently shown
    help_modal_shown: bool,
    /// Action to be completed once the current modal is closed.
    modal_action: Option<ModalAction>,
    /// Whether unsaved changes should be ignored when closing the editor
    ignore_dirty: bool,
    /// Handle to the backend when in the web editor
    #[cfg(target_arch = "wasm32")]
    backend_handle: platform::BackendHandle,
}

impl eframe::App for App {
    // perform all the editor logic by updating it each frame
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // check whether the user has requested to close
        // the window while they have unsaved changes
        if ctx.input(|i| i.viewport().close_requested())
            && !self.ignore_dirty
            && self.buffers.is_dirty()
        {
            // show the modal for unsaved changes, and cancel the window closing
            self.save_modal_state = SaveModalState::SaveAllOpen;
            self.modal_action = Some(ModalAction::Close);
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        }

        self.handle_shortcuts(ctx);

        // display menu bar
        TopBottomPanel::top("top_menu_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.visuals_mut().button_frame = false;
                self.menu_bar(ctx, ui);
            });
        });

        // display side panel showing the file tree Explorer 
        let max_left_panel_width = 0.8;
        if let Some(explorer) = self.explorer.as_mut() {
            let response = SidePanel::left("explorer_panel")
                .resizable(true)
                .max_width(max_left_panel_width * ctx.available_rect().width())
                .show(ctx, |ui| explorer.show(ui, &self.fs));

            if response.response.clicked_elsewhere() {
                explorer.highlighted = None;
            }

            match response.inner {
                Ok(explorer_response) => {
                    if let Some(action) = explorer_response.action {
                        match action {
                            ExplorerAction::OpenFile(path) => self.open_file(path),
                            // TODO: the 2 below
                            ExplorerAction::NewFolder(path) => todo!(), //self.new_folder(path),
                            ExplorerAction::Delete(path) => todo!(),    // self.delete_file(path),
                        }
                    }
                }
                Err(err) => self.error_message = Some(err.to_string()),
            }
        }

        // if Delete key pressed while selecting a file in the explorer, then delete the file
        if let Some(path) = self.explorer.as_ref().and_then(|e| e.highlighted.clone())
            && ctx.input(|i| i.key_pressed(Key::Delete))
        {
            self.delete(path.as_path());
        }

        if let Some(bottom_panel_state) = self.bottom_panel_state {
            let max_bottom_panel_height = 0.8;
            TopBottomPanel::bottom("bottom_panel")
                .resizable(true)
                .max_height(max_bottom_panel_height * ctx.available_rect().height())
                .show(ctx, |ui| {
                    let size = ui.available_size();
                    match bottom_panel_state {
                        BottomPanelState::Output => self.output(ui, size),
                        BottomPanelState::Terminal => self.terminal(ui, size),
                    }
                });
        }

        let buffers_response = CentralPanel::default()
            .show(ctx, |ui| {
                self.buffers
                    .show(&self.editor_settings, ui, &self.code_theme, &self.fs)
            })
            .inner;
        if let Some(action) = buffers_response.save_modal_action {
            self.save_modal_state = SaveModalState::SaveFileOpen;
            self.modal_action = Some(action);
        }
        if let Some(err) = buffers_response.error_message {
            self.error_message = Some(err);
        }

        let mut changed = false;
        let mut replaced = false;
        let mut done = false;
        let mut opened = None;
        if let Some(search_state) = &mut self.search_modal_state {
            Self::show_search_modal(
                ctx,
                search_state,
                &mut opened,
                &mut changed,
                &mut replaced,
                &mut done,
            );

            if let Some(ref loc) = opened
                && let Some(buffer) = self.buffers.get_by_path(&loc.path)
            {
                self.buffers.select(buffer.id());
            }
        }

        if let Some(search_state) = &self.search_modal_state {
            if replaced {
                let paths: Vec<_> = search_state
                    .search_results
                    .iter()
                    .map(|r| r.path.as_path())
                    .collect();
                let _ = self.fs.replace(
                    &paths,
                    &search_state.search_text,
                    &search_state.replace_text,
                );
            }

            if changed {
                self.update_search_results(&search_state.search_text.clone());
            }
        }
        if done || opened.is_some() {
            self.search_modal_state = None;
        }

        if self.settings_modal_state.is_some() {
            self.show_settings_modal(ctx);
        }

        if self.help_modal_shown {
            self.show_help_modal(ctx);
        }

        #[cfg(not(target_arch = "wasm32"))]
        match self.save_modal_state {
            SaveModalState::SaveAllOpen => self.show_save_modal(ctx, true),
            SaveModalState::SaveFileOpen => self.show_save_modal(ctx, false),
            SaveModalState::Closing => {
                self.save_modal_state = SaveModalState::Closed;
            }
            SaveModalState::Closed => {
                if let Some(action) = self.modal_action.take() {
                    self.modal_action(action, ctx);
                }
            }
        }

        if self.error_message.is_some() {
            self.show_error_modal(ctx);
        }

        self.runner.update();

        #[cfg(target_arch = "wasm32")]
        self.handle_pending();
    }
}

impl App {
    #[cfg(target_arch = "wasm32")]
    pub async fn new(user: String, repo: String) -> Self {
        let project = platform::Project::new(user, repo)
            .await
            .expect("failed to create project");
        let fs = platform::FileSystem::new(project.handle().clone());
        let runner = platform::Runner::new(project.handle().clone());
        let backend_handle = project.handle().clone();

        Self {
            project: Some(project),
            fs,
            runner,
            backend_handle,
            ..Self::default()
        }
    }

    // displays the menu bar at the top of the screen
    fn menu_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.button("New file").clicked() {
                        self.buffers.add(Buffer::empty());
                    }
                    ui.separator();
                    if ui.button("Open file").clicked() {
                        self.open_file_dialog();
                    }
                    if ui.button("Open folder").clicked() {
                        self.open_folder(ctx);
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.separator();

                        // Enable search and replace if there is a currently open project
                        if ui
                            .add_enabled(self.project.is_some(), Button::new("Search"))
                            .clicked()
                        {
                            self.search_modal_state = Some(SearchModalState::default());
                        }
                        if ui
                            .add_enabled(self.project.is_some(), Button::new("Replace"))
                            .clicked()
                        {
                            self.search_modal_state = Some(SearchModalState {
                                is_replace: true,
                                ..Default::default()
                            });
                        }
                    }

                    ui.separator();

                    // only show "Save file"/"Save as..." if there is a currently selected buffer
                    let show_save = self.buffers.current_buffer().is_some();
                    if ui
                        .add_enabled(show_save, Button::new("Save file"))
                        .clicked()
                    {
                        match self.save_file() {
                            Err(SaveError::NoBufferSelected) => {
                                self.error_message = Some("Failed to save file".into())
                            }
                            Ok(_) | Err(SaveError::NoFileSelected) => {} // do nothing if the user doesn't selected a file to save to
                        }
                    }
                    if ui
                        .add_enabled(show_save, Button::new("Save as..."))
                        .clicked()
                    {
                        if let Err(SaveError::NoBufferSelected) = self.save_as() {
                            unreachable!()
                        }
                    }
                    // only show Save all if any of the buffers are have unsaved changes
                    let show_save_all = self.buffers.is_dirty();
                    if ui
                        .add_enabled(show_save_all, Button::new("Save all changes"))
                        .clicked()
                    {
                        self.save_all();
                    }
                }
                // Save to GitHub is only available on the web
                #[cfg(target_arch = "wasm32")]
                {
                    if ui.button("Save to GitHub").clicked() {
                        self.save_to_github();
                    }
                }
            });
            ui.menu_button("Edit", |ui| {
              // open the settings modal if Settings is clicked
              if ui.button("Settings").clicked() {
                    self.settings_modal_state = Some(EditorSettings::default());
                }
            });
            ui.menu_button("View", |ui| {
                // add toggle buttons for Output and Terminal panels
                
                if ui
                    .add_enabled(self.explorer.is_some(), Button::new("Show output"))
                    .clicked()
                {
                    if let Some(BottomPanelState::Output) = self.bottom_panel_state {
                        self.bottom_panel_state = None;
                    } else {
                        self.bottom_panel_state = Some(BottomPanelState::Output)
                    }
                }
                if ui
                    .add_enabled(self.explorer.is_some(), Button::new("Show terminal"))
                    .clicked()
                {
                    if let Some(BottomPanelState::Terminal) = self.bottom_panel_state {
                        self.bottom_panel_state = None;
                    } else {
                        self.bottom_panel_state = Some(BottomPanelState::Terminal)
                    }
                }
            });

            ui.menu_button("Run", |ui| {
                let show_run = self.project.is_some();
                if ui.add_enabled(show_run, Button::new("Run")).clicked() {
                    if let Err(e) = self.run() {
                        self.error_message = Some(e.to_string());
                    }
                }
            });
            ui.menu_button("Help", |ui| {
                if ui.button("Help").clicked() {
                    self.help_modal_shown = true;
                }
            });

            #[cfg(target_arch = "wasm32")]
            if ui.button("Quit").clicked() {
                // FIXME: confirmation (also for when tab is closed)
                // suggest git committing
                web_sys::window().unwrap().location().set_href("/").unwrap();
            }

            self.running_buttons(ui);
        });
    }

    // Display Stop button to stop execution if the project is currently running 
    fn running_buttons(&mut self, ui: &mut egui::Ui) {
        ui.scope(|ui| {
            ui.style_mut().visuals.button_frame = true;
            if self.runner.is_running() && ui.button("Stop").clicked() {
                self.runner.stop();
            }
        });
    }

    // display program output in a scrollable monospaced text box
    fn output(&self, ui: &mut egui::Ui, size: egui::Vec2) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_sized(
                size,
                egui::TextEdit::multiline(
                    &mut self.output.lock().expect("failed to get output").as_str(),
                )
                .desired_width(f32::INFINITY)
                .code_editor(),
            );
        });
    }

    // display terminal panel
    fn terminal(&mut self, ui: &mut egui::Ui, size: egui::Vec2) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(terminal) = &mut self.terminal {
            let view = TerminalView::new(ui, terminal).set_size(size);
            ui.add(view);
        }
    }

    // Saves the current contents of the code buffer to a file.
    //
    // If no file is currently associated with the `App`, it prompts the user
    // to select a save location. Once a file is chosen or if a file already
    // exists, it writes the contents of the code buffer to the file.
    // It updates the `file` field with the latest contents after saving.
    //
    // Returns `true` if the save was completed.
    #[cfg(not(target_arch = "wasm32"))]
    fn save_file(&mut self) -> Result<(), SaveError> {
        match self.buffers.current_buffer_mut() {
            Some(buffer) => match buffer.save(&self.fs) {
                Ok(_) => Ok(()),
                Err(_) => self.save_as(),
            },
            None => Err(SaveError::NoBufferSelected),
        }
    }

    // Saves the selected buffer into a new file.
    // Prompts the user to select the new location of the file.
    //
    // Returns `true` if the save was completed.
    #[cfg(not(target_arch = "wasm32"))]
    fn save_as(&mut self) -> Result<(), SaveError> {
        let Some(buffer) = self.buffers.current_buffer() else {
            return Err(SaveError::NoBufferSelected);
        };

        let Some(path) = rfd::FileDialog::new().save_file() else {
            return Err(SaveError::NoFileSelected);
        };

        // If the current buffer has no file, associate it with the file
        // Otherwise, create a new buffer with the new file (unless the file hasn't changed)
        match buffer.file_data() {
            Some(FileData { path: old_path, .. }) => {
                if &path != old_path {
                    self.buffers.add(Buffer::new(
                        buffer.contents().into(),
                        Some(FileData {
                            path,
                            contents: buffer.contents().into(),
                        }),
                    ));
                }
            }
            None => {
                let buffer = self.buffers.current_buffer_mut().unwrap();
                buffer.set_file_data(FileData {
                    path,
                    contents: buffer.contents().into(),
                });
            }
        }
        self.save_file()?;

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_all(&mut self) {
        // collect list of buffers with unsaved changes
        let dirty_buffers: Vec<_> = self
            .buffers
            .iter()
            .filter_map(|buf| buf.is_dirty().then_some(buf.id()))
            .collect();

        // loop through unsaved buffers, and prompt the user to save each one
        for id in dirty_buffers {
            // unwrap is safe here as `id` is guaranteed to be associated with a buffer
            let buffer = self.buffers.get_mut_by_id(id).unwrap();
            
            if let Err(BufferError::NoAssociatedFile) = buffer.save(&self.fs) {
                self.buffers.select(id);
                // if no file is selected, ignore it and continue saving all
                let _ = self.save_as();
            }
        }
    }

    // send a request to the backend API at /project/github_save
    // to commit the contents of the project to GitHub
    #[cfg(target_arch = "wasm32")]
    fn save_to_github(&mut self) {
        wasm_bindgen_futures::spawn_local(async move {
            gloo_net::http::Request::post("/api/project/github_save")
                .send()
                .await
                .expect("failed to save project to github");
        });
    }

    // open OS-provided dialog to select a file to open
    #[cfg(not(target_arch = "wasm32"))]
    fn open_file_dialog(&mut self) {
        let Some(path) = rfd::FileDialog::new().pick_file() else {
            log::info!("no file selected to open");
            return;
        };

        self.open_file(path);
    }

    // open a new buffer for the file at the provided path
    fn open_file(&mut self, path: PathBuf) {
        // Don't open a new tab if the file is already open
        if let Some(buffer) = self.buffers.get_by_path(&path) {
            self.buffers.select(buffer.id());
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match Buffer::from_path(path, &self.fs) {
                Ok(buffer) => self.buffers.add(buffer),
                Err(err) => self.error_message = Some(err.to_string()),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = self.fs.read_file(&path);
        }
    }

    // choose a new folder to open as the current project
    #[cfg(not(target_arch = "wasm32"))]
    fn open_folder(&mut self, ctx: &egui::Context) {
        if !self.ignore_dirty && self.buffers.is_dirty() {
            self.save_modal_state = SaveModalState::SaveAllOpen;
            self.modal_action = Some(ModalAction::OpenFolder);
            return;
        }

        let Some(path) = rfd::FileDialog::new().pick_folder() else {
            log::info!("no folder selected to open");
            return;
        };

        self.open_project(ctx, path);
    }

    // setup a new project, rooted at the provided directory path
    #[cfg(not(target_arch = "wasm32"))]
    fn open_project(&mut self, ctx: &egui::Context, path: PathBuf) {
        use egui_term::{BackendSettings, TerminalBackend};

        use crate::platform::{Project, ProjectSettings};

        // verify that the path is a directory
        if !path.is_dir() {
            panic!("path must be a dir");
        }

        // load the project settings from the .ide directory
        let settings = match ProjectSettings::read_from(&path) {
            Ok(settings) => settings,
            Err(err) => {
                self.error_message = Some(err.to_string());
                return;
            }
        };

        // instantiate a new `Project` object
        self.project = Some(Project::new(path.clone(), settings));

        // initialise the interactive terminal backend
        // requires using an MPSC (Multiple Producer, Single Consumer) channel to send data between
        // the process running the terminal shell and the widget displaying the terminal in the UI
        let (sender, receiver) = std::sync::mpsc::channel();
        let shell = if cfg!(target_os = "windows") {
            "cmd.exe".to_string()
        } else {
            std::env::var("SHELL").expect("SHELL is not defined")
        };
        self.terminal = Some(
            TerminalBackend::new(
                0,
                ctx.clone(),
                sender,
                BackendSettings {
                    shell,
                    ..Default::default()
                },
            )
            .expect("failed to create terminal"),
        );
        // FIXME: this might be to do with why terminal won't work (should this receiver be used somewhere?)
        Box::leak(Box::new(receiver));

        // create a new Explorer side panel
        match Explorer::new(path, &self.fs) {
            Ok(explorer) => {
                self.explorer = Some(explorer);
                self.buffers = Buffers::default();
            }
            // display error message to user if loading file tree failed
            Err(err) => self.error_message = Some(err.to_string()),
        }
    }

    // delete the file for the path, and remove the buffer in the UI
    fn delete(&mut self, path: &Path) {
        if let Some(buffer) = self.buffers.get_by_path(path) {
            self.buffers.delete_buffer(buffer.id());
        }

        if let Err(err) = self.fs.delete(path) {
            self.error_message = Some(err.to_string());
        }
    }

    // sets the color scheme of the editor
    fn set_color_scheme(&mut self, ctx: &egui::Context, scheme: &ColorScheme) {
        ctx.set_style(AvailableColorSchemes::scheme_to_style(scheme));
    }

    // Resume the action that was being performed before a modal was displayed
    //
    // E.g. if the user tries to open a new project when they have unsaved changes,
    // then, once the user closes the modal, this method will be called `ModalAction::OpenFolder`
    fn modal_action(&mut self, action: ModalAction, ctx: &egui::Context) {
        self.ignore_dirty = false;
        self.modal_action = None;

        #[cfg(not(target_arch = "wasm32"))]
        match action {
            ModalAction::OpenFile => self.open_file_dialog(),
            ModalAction::OpenFolder => self.open_folder(ctx),
            ModalAction::DeleteBuffer(id) => self.buffers.delete_buffer(id),
            ModalAction::Close => {
                self.ignore_dirty = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    // Shows a modal prompting the user to save any unsaved changes.
    //
    // If an action was taking place before the modal was opened (`self.modal_action` is `Some`), it is executed after the modal is closed.
    #[cfg(not(target_arch = "wasm32"))]
    fn show_save_modal(&mut self, ctx: &egui::Context, save_all: bool) {
        Modal::new(Id::new("confirm_unsaved_changes")).show(ctx, |ui| {
            ui.label("You have unsaved changes, do you want to save them?");
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    if save_all {
                        self.save_all();
                        self.save_modal_state = SaveModalState::Closing;
                    } else {
                        match self.save_file() {
                            Err(SaveError::NoFileSelected) => {}
                            _ => self.save_modal_state = SaveModalState::Closing,
                        }
                    }
                }
                if ui.button("Don't save").clicked() {
                    self.ignore_dirty = true;
                    self.save_modal_state = SaveModalState::Closing;
                }
                if ui.button("Cancel").clicked() {
                    self.modal_action = None;
                    self.save_modal_state = SaveModalState::Closing;
                }
            })
        });
    }

    // Show a modal displaying editable drop downs and toggles for adjusting the editor settings
    fn show_settings_modal(&mut self, ctx: &egui::Context) {
        let mut updated = false;
        Modal::new(Id::new("settings_modal")).show(ctx, |ui| {
            let settings_state = self.settings_modal_state.as_mut().unwrap();

            ui.label("Settings");

            ui.checkbox(&mut settings_state.auto_save, "Auto-save");
            ui.checkbox(&mut settings_state.format_on_save, "Format on save");
            
            // color schemes
            ComboBox::from_label("Color Scheme")
                .selected_text(settings_state.color_scheme.clone().unwrap_or_default())
                .show_ui(ui, |ui| {
                    for scheme in self.available_color_schemes.schemes.iter() {
                        ui.selectable_value(
                            &mut settings_state.color_scheme,
                            Some(scheme.name().to_string()),
                            scheme.name(),
                        );
                    }
                });

            // TODO: add options for other settings
            
            ui.horizontal(|ui| {
                if ui.button("Done").clicked() {
                    updated = true;
                }
            });
        });

        if updated {
            self.update_settings(ctx);
        }
    }

    fn update_settings(&mut self, ctx: &egui::Context) {
        let settings = self.settings_modal_state.take().unwrap();

        if let Some(scheme) = &settings.color_scheme {
            let scheme = self.available_color_schemes.get_scheme(scheme).unwrap();
            self.set_color_scheme(ctx, &scheme.clone());
        }

        #[cfg(target_arch = "wasm32")]
        self.backend_handle
            .send(ws_messages::Command::UpdateSettings {
                settings: settings.clone(),
            });

        self.editor_settings = settings;
    }

    // show modal which allows the user to perform search and replace
    fn show_search_modal(
        ctx: &egui::Context,
        search_state: &mut SearchModalState,
        opened: &mut Option<SearchResult>,
        changed: &mut bool,
        replaced: &mut bool,
        done: &mut bool,
    ) {
        Modal::new(Id::new("search_modal")).show(ctx, |ui| {
            ui.label("Search");
            // text prompt for search query
            if ui
                .text_edit_singleline(&mut search_state.search_text)
                .changed()
            {
                *changed = true;
            }
            // toggle for replace mode
            ui.checkbox(&mut search_state.is_replace, "Replace");
            if search_state.is_replace {
                // text prompt for replace string
                ui.text_edit_singleline(&mut search_state.replace_text);
                ui.horizontal(|ui| {
                    if ui.button("Replace all").clicked() {
                        *replaced = true;
                        *done = true;
                    }

                    if ui.button("Done").clicked() {
                        *done = true;
                    }
                });
            } else {
                ui.horizontal(|ui| {
                    if ui.button("Done").clicked() {
                        *done = true;
                    }
                });
            }

            ui.separator();

            // display search results in rows, which can be clicked to open the file
            Grid::new("search_results")
                .striped(true)
                .num_columns(1)
                .show(ui, |ui| {
                    for res in &search_state.search_results {
                        if ui
                            .add(Button::new(format!(
                                "{}:{}:{}",
                                res.path.to_string_lossy(),
                                res.line,
                                res.col
                            )))
                            .clicked()
                        {
                            *opened = Some(res.clone());
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn update_search_results(&mut self, search: &str) {
        if let Some(project) = &self.project
            && let Some(search_state) = &mut self.search_modal_state
        {
            let results = self.fs.search_project(project, search);
            search_state.search_results = results;
        }
    }

    // display an error message to the user
    fn show_error_modal(&mut self, ctx: &egui::Context) {
        let modal = Modal::new(Id::new("error_modal")).show(ctx, |ui| {
            ui.label(self.error_message.as_deref().unwrap_or("An error occurred"));
            ui.with_layout(Layout::default().with_cross_align(Align::Max), |ui| {
                // clicking the OK button closes the modal
                if ui.button("OK").clicked() {
                    self.error_message = None;
                }
            });
        });

        if modal.should_close() {
            self.error_message = None;
        }
    }

    fn show_help_modal(&mut self, ctx: &egui::Context) {
        let modal = Modal::new(Id::new("help_modal")).show(ctx, |ui| {
            ui.label("Help");

            ui.separator();

            ui.label(RichText::new("Keyboard shortcuts:").underline());
            
            ui.label("Ctrl + S: Save file\n\
                Ctrl + Shift + S: Save as\n\
                Ctrl + Alt + S: Save all\n\
                Ctrl + N: New file\n\
                Ctrl + O: Open file\n\
                Ctrl + Shift + O: Open folder\n\
                Ctrl + F: Search\n\
                Ctrl + H: Replace\n\
                Ctrl + ,: Settings\n\
                Ctrl + `: Toggle terminal\n\
                Ctrl + Shift + U: Toggle output\n\
                F5: Run",
            );
        });

        if modal.should_close() {
            self.help_modal_shown = false;
        }
    }

    fn show_help_modal(&mut self, ctx: &egui::Context) {
        let modal = Modal::new(Id::new("help_modal")).show(ctx, |ui| {
            ui.label("Help");

            ui.separator();

            ui.label(RichText::new("Keyboard shortcuts:").underline());
            
            ui.label("Ctrl + S: Save file\n\
                Ctrl + Shift + S: Save as\n\
                Ctrl + Alt + S: Save all\n\
                Ctrl + N: New file\n\
                Ctrl + O: Open file\n\
                Ctrl + Shift + O: Open folder\n\
                Ctrl + F: Search\n\
                Ctrl + H: Replace\n\
                Ctrl + ,: Settings\n\
                Ctrl + `: Toggle terminal\n\
                Ctrl + Shift + U: Toggle output\n\
                F5: Run",
            );
        });

        if modal.should_close() {
            self.help_modal_shown = false;
        }
    }

    // Stop the current program if running and start a new execution 
            // TODO: error/test cases in the NEA write-up should include all of the `ok_or_eyre` and `bail!` errors in this function
    fn run(&mut self) -> eyre::Result<()> {
        self.runner.stop();

        self.bottom_panel_state = Some(BottomPanelState::Output);

        let project = self.project.as_mut().ok_or_eyre("No project open")?;

        self.runner.run(project, self.output.clone())?;

        Ok(())
    }

    // update editor based on messages received from server over websocket
    #[cfg(target_arch = "wasm32")]
    fn handle_pending(&mut self) {
        use ws_messages::{Command::*, ProjectTree, Response::*, RunAction};

        // TODO: document what this call does (rn i cba checking the platform/ dir)
        self.backend_handle.update();
                
        // iterate through all received websocket messages
        for resp in self.backend_handle.responses() {
            use std::io::Read;

            // TODO: maybe explain what each and every of these commands do
            
            // pattern match agaisnt possible sent commands and their received response
            match resp.expect("FIXME: proper error handling") {
                (OpenProject, Project { contents, settings }) => {
                    self.editor_settings = settings;

                    let path = contents.path().clone();
                    self.fs.cache(contents);
                    log::info!("opened project: {}", path.display());

                    self.explorer = Some(Explorer::new(path, &self.fs).unwrap());
                }
                (ReadSettings { action }, ProjectSettings { contents }) => {
                    if contents.is_empty() {
                        self.error_message = Some("No settings found".to_string());
                        continue;
                    }
                    let settings = match platform::ProjectSettings::from_contents(&contents) {
                        Ok(settings) => settings,
                        Err(err) => {
                            self.error_message = Some(err.to_string());
                            continue;
                        }
                    };
                    self.runner.run_action(&settings, action);
                    self.project.as_mut().unwrap().set_settings(settings);
                }
                (ColorSchemes, AvailableSchemes { color_schemes }) => {
                    self.available_color_schemes = AvailableColorSchemes {
                        schemes: color_schemes,
                    };
                }
                (ReadFile { path }, FileContents { contents }) => self.buffers.add(Buffer::new(
                    contents.clone(),
                    Some(FileData {
                        contents,
                        path: path.clone(),
                    }),
                )),
                (ReadDir { path }, DirContents { contents_paths }) => {}
                (Run { .. }, Output { output }) => {
                    self.runner.set_finished();
                    self.output.lock().unwrap().push_str(&output);
                }
                (_, Success) => {}
                // the server sent an invalid response to the RPC call
                _ => {
                    panic!("FIXME: error handle here or something")
                }
            }
        }
    }
}
