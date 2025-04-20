use crate::{
    buffer::{Buffer, BufferError, Buffers, FileData},
    explorer::Explorer,
    pipe_reader::{read_piped, PipedLine},
    project::{Project, ProjectSettings},
};

use core::f32;
use std::{
    path::PathBuf,
    process::{Child, Stdio},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

use crossbeam_channel as crossbeam;
use eframe::egui;
use egui::{
    containers::modal::Modal, vec2, Align, Button, CentralPanel, Id, Layout, ScrollArea, SidePanel, TopBottomPanel, ViewportCommand
};
use egui_extras::syntax_highlighting;
use eyre::{bail, OptionExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub enum ModalAction {
    OpenFile,
    OpenFolder,
    DeleteBuffer(Uuid),
    Close,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EditorSettings {
    pub auto_save: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self { auto_save: true }
    }
}

#[derive(Debug, Clone, Copy)]
enum SaveError {
    NoFileSelected,
    NoBufferSelected,
}

/// The current state of the save modal
#[derive(Default, Debug, Serialize, Deserialize)]
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

#[derive(Debug)]
struct RunningCommand {
    process: Child,
    thread: JoinHandle<()>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct App {
    editor_settings: EditorSettings,
    project: Option<Project>,
    #[serde(skip)]
    running_command: Option<RunningCommand>,
    buffers: Buffers,
    /// Current [`syntax_highlighting::CodeTheme`] for the editor
    code_theme: syntax_highlighting::CodeTheme,
    /// [`Explorer`] side panel
    explorer: Option<Explorer>,
    /// Contents of the output panel
    /// This must be wrapped in an `Arc<Mutex<_>>` so that it can be shared to and modified across threads, including the `running_command` thread.
    output: Arc<Mutex<String>>,
    #[serde(skip)]
    error_message: Option<String>,
    /// Current state of the save modal, as described in [`SaveModalState`]
    #[serde(skip)]
    save_modal_state: SaveModalState,
    /// Action to be completed once the current modal is closed.
    #[serde(skip)]
    modal_action: Option<ModalAction>,
    #[serde(skip)]
    ignore_dirty: bool,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested())
            && !self.ignore_dirty
            && self.buffers.is_dirty()
        {
            self.save_modal_state = SaveModalState::SaveAllOpen;
            self.modal_action = Some(ModalAction::Close);
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
        }

        TopBottomPanel::top("top_menu_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.visuals_mut().button_frame = false;
                self.menu_bar(ui);
            });
        });

        let max_left_panel_width = 0.8;
        if let Some(explorer) = self.explorer.as_mut() {
            let explorer_response = SidePanel::left("explorer_panel")
                .resizable(true)
                .max_width(max_left_panel_width * ctx.available_rect().width())
                .show(ctx, |ui| explorer.show(ui))
                .inner;

            match explorer_response {
                Ok(explorer_response) => {
                    if let Some(path) = explorer_response.open_file {
                        self.open_file(Some(path));
                    }
                }
                Err(err) => self.error_message = Some(err.to_string()),
            }
        }

        let max_bottom_panel_height = 0.8;
        TopBottomPanel::bottom("output_panel")
            .resizable(true)
            .max_height(max_bottom_panel_height * ctx.available_rect().height())
            .show(ctx, |ui| {
                let size = ui.available_size();
                self.output(ui, size);
            });

        let buffers_response = CentralPanel::default()
            .show(ctx, |ui| {
                self.buffers
                    .show(&self.editor_settings, ui, &self.code_theme)
            })
            .inner;
        if let Some(action) = buffers_response.save_modal_action {
            self.save_modal_state = SaveModalState::SaveFileOpen;
            self.modal_action = Some(action);
        }
        if let Some(err) = buffers_response.error_message {
            self.error_message = Some(err);
        }

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

        if self
            .running_command
            .as_ref()
            .is_some_and(|cmd| cmd.thread.is_finished())
        {
            self.running_command = None;
        }
    }
}

impl App {
    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New file").clicked() {
                    self.buffers.add(Buffer::empty());
                }
                ui.separator();
                if ui.button("Open file").clicked() {
                    self.open_file(None);
                }
                if ui.button("Open folder").clicked() {
                    self.open_folder();
                }
                ui.separator();

                let show_save = self.buffers.current_buffer().is_some();
                if ui
                    .add_enabled(show_save, Button::new("Save file"))
                    .clicked()
                {
                    // this button should be greyed out if no buffer is selected, so we should be able to unwrap here
                    self.save_file().unwrap();
                }
                if ui
                    .add_enabled(show_save, Button::new("Save as..."))
                    .clicked()
                {
                    if let Err(SaveError::NoBufferSelected) = self.save_as() {
                        unreachable!()
                    }
                }
                let show_save_all = self.buffers.is_dirty();
                if ui
                    .add_enabled(show_save_all, Button::new("Save all changes"))
                    .clicked()
                {
                    self.save_all();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo").clicked() {
                    todo!("undo");
                }
                if ui.button("Redo").clicked() {
                    todo!("redo");
                }
                ui.separator();
                if ui.button("Settings").clicked() {}
            });
            ui.menu_button("View", |_ui| {});
            ui.menu_button("Run", |ui| {
                if ui.button("Run").clicked() {
                    if let Err(e) = self.run() {
                        self.error_message = Some(e.to_string());
                    }
                }
            });
            ui.menu_button("Help", |_ui| {});

            self.running_buttons(ui);
        });
    }

    fn running_buttons(&mut self, ui: &mut egui::Ui) {
        ui.scope(|ui| {
            ui.style_mut().visuals.button_frame = true;
            if self.running_command.is_some() && ui.button("Stop").clicked() {
                self.stop();
            }
        });
    }

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

    /// Saves the current contents of the code buffer to a file.
    ///
    /// If no file is currently associated with the `App`, it prompts the user
    /// to select a save location. Once a file is chosen or if a file already
    /// exists, it writes the contents of the code buffer to the file.
    /// It updates the `file` field with the latest contents after saving.
    ///
    /// Returns `true` if the save was completed.
    fn save_file(&mut self) -> Result<(), SaveError> {
        match self.buffers.current_buffer_mut() {
            Some(buffer) => match buffer.save() {
                Ok(_) => Ok(()),
                Err(_) => self.save_as(),
            },
            None => Err(SaveError::NoBufferSelected),
        }
    }

    // TODO: doc comment
    ///
    /// Returns `true` if the save was completed.
    fn save_as(&mut self) -> Result<(), SaveError> {
        let Some(buffer) = self.buffers.current_buffer() else {
            return Err(SaveError::NoBufferSelected);
        };

        let Some(path) = rfd::FileDialog::new().save_file() else {
            return Err(SaveError::NoFileSelected);
        };

        // If the current buffer has no file, associate it with the file
        // Otherwise, create a new buffer with the new file (unless the file hasn't changed)
        match &buffer.file_data {
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

    fn save_all(&mut self) {
        let dirty_buffers: Vec<_> = self
            .buffers
            .iter()
            .filter_map(|buf| buf.is_dirty().then_some(buf.id()))
            .collect();

        for id in dirty_buffers {
            // unwrap is safe here as `id` is guaranteed to be associated with a buffer
            let buffer = self.buffers.get_mut_by_id(id).unwrap();
            if let Err(BufferError::NoAssociatedFile) = buffer.save() {
                self.buffers.select(id);
                // if no file is selected, ignore it and continue saving all
                // TODO: make it so the ui updates (to show which tab is selected) in between individual calls to `save_as`
                let _ = self.save_as();
            }
        }
    }

    fn open_file(&mut self, path: Option<PathBuf>) {
        let Some(path) = path.or_else(|| rfd::FileDialog::new().pick_file()) else {
            log::info!("no file selected to open");
            return;
        };

        // Don't open a new tab if the file is already open
        if let Some(buffer) = self.buffers.get_by_path(&path) {
            self.buffers.select(buffer.id());
            return;
        }

        match Buffer::from_path(path) {
            Ok(buffer) => self.buffers.add(buffer),
            Err(err) => self.error_message = Some(err.to_string()),
        }
    }

    fn open_folder(&mut self) {
        if !self.ignore_dirty && self.buffers.is_dirty() {
            self.save_modal_state = SaveModalState::SaveAllOpen;
            self.modal_action = Some(ModalAction::OpenFolder);
            return;
        }

        let Some(path) = rfd::FileDialog::new().pick_folder() else {
            log::info!("no folder selected to open");
            return;
        };

        self.open_project(path);
    }

    fn open_project(&mut self, path: PathBuf) {
        let settings = match ProjectSettings::read_from(&path) {
            Ok(settings) => settings,
            Err(err) => {
                self.error_message = Some(err.to_string());
                return;
            }
        };

        self.project = Some(Project {
            path: path.clone(),
            settings,
        });

        match Explorer::new(path) {
            Ok(explorer) => {
                self.explorer = Some(explorer);
                self.buffers = Buffers::default();
            }
            Err(err) => self.error_message = Some(err.to_string()),
        }
    }

    fn modal_action(&mut self, action: ModalAction, ctx: &egui::Context) {
        self.ignore_dirty = false;
        self.modal_action = None;
        match action {
            ModalAction::OpenFile => self.open_file(None),
            ModalAction::OpenFolder => self.open_folder(),
            ModalAction::DeleteBuffer(id) => self.buffers.delete_buffer(id),
            ModalAction::Close => {
                self.ignore_dirty = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
        }
    }

    /// Shows a modal prompting the user to save any unsaved changes.
    ///
    /// If an action was taking place before the modal was opened (`self.modal_action` is `Some`), it is executed after the modal is closed.
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

    fn show_error_modal(&mut self, ctx: &egui::Context) {
        let modal = Modal::new(Id::new("error_modal")).show(ctx, |ui| {
            ui.label(self.error_message.as_deref().unwrap_or("An error occurred"));
            ui.with_layout(Layout::default().with_cross_align(Align::Max), |ui| {
                if ui.button("OK").clicked() {
                    self.error_message = None;
                }
            });
        });

        if modal.should_close() {
            self.error_message = None;
        }
    }

    // TODO: error cases in the NEA write-up should include all of the `ok_or_eyre` and `bail!` errors in this function
    fn run(&mut self) -> eyre::Result<()> {
        self.stop();

        // TODO: don't show missing/invalid run command errors as modals that take up the whole screen

        // TODO: maybe gray out the run button if this is the case
        let project = self.project.as_mut().ok_or_eyre("No project open")?;

        // update project settings
        match ProjectSettings::read_from(&project.path) {
            Ok(settings) => project.settings = settings,
            Err(err) => Err(err)?,
        }

        let settings = project
            .settings
            .as_ref()
            .ok_or_eyre("No run command set\n\nA project.toml file is needed to set it")?;

        let mut words = match shell_words::split(&settings.run_command) {
            Ok(words) => words.into_iter(),
            Err(_) => bail!("Invalid run command"),
        };
        let command = words.next().ok_or_eyre("Run command is empty")?;

        let args = words.collect::<Vec<String>>();

        // TODO: error handling for this should include handling "program not found" and "invalid input"
        let mut child = std::process::Command::new(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&project.path)
            .args(args)
            .spawn()
            .expect("failed to start subprocess");
        

        self.output.lock().expect("failed to lock output").clear();
        let output = self.output.clone();

        // should be able to unwrap these, as we set stdout and stderr in the Command
        let out = read_piped(child.stdout.take().unwrap());
        let err = read_piped(child.stderr.take().unwrap());

        let thread = thread::spawn(move || {
            loop {
                // waits to receive the next line from either stdout or stderr, and processes which ever one is received first
                crossbeam::select! {
                    recv(out) -> msg => match msg {
                        Ok(Ok(PipedLine::Line(line))) => {
                            println!("{:?}", &line);
                            output.lock().expect("failed to lock output").push_str(&(line));
                        }
                        Ok(Ok(PipedLine::Eof)) | Err(_) => break,
                        // TODO: handle this error
                        Ok(Err(err)) => eprintln!("Error: {:?}", err),
                    },
                    recv(err) -> msg => match msg {
                        Ok(Ok(PipedLine::Line(line))) => output.lock().expect("failed to lock output").push_str(&format!("** {} **\n", &line)),
                        Ok(Ok(PipedLine::Eof)) | Err(_) => break,
                        Ok(Err(err)) => eprintln!("Error: {:?}", err),
                    },
                }
            }
        });

        self.running_command = Some(RunningCommand {
            process: child,
            thread,
        });

        Ok(())
    }

    fn stop(&mut self) {
        if let Some(mut running_command) = self.running_command.take() {
            running_command
                .process
                .kill()
                .expect("failed to kill process");
            running_command
                .thread
                .join()
                .expect("failed to join thread");
        }
    }
}
