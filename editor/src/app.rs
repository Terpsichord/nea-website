use crate::{
    buffer::{Buffer, BufferError, Buffers, FileData},
    explorer::Explorer,
};

use core::f32;
use std::{
    fmt::Debug,
    ops::{Deref, DerefMut}, path::PathBuf,
};

use eframe::egui;
use egui::{containers::modal::Modal, text_edit::TextEditState, Button, Id};
use egui_extras::{Size, StripBuilder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub enum ModalAction {
    OpenFile,
    DeleteBuffer(Uuid),
    Close,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub auto_save: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { auto_save: true }
    }
}

#[derive(Serialize, Deserialize)]
struct EditorState(TextEditState);

impl From<TextEditState> for EditorState {
    fn from(state: TextEditState) -> Self {
        Self(state)
    }
}

impl Deref for EditorState {
    type Target = TextEditState;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EditorState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for EditorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorState").finish()
    }
}

enum SaveError {
    NoFileSelected,
    NoBufferSelected,
}

#[derive(Default, Debug, Serialize, Deserialize)]
enum SaveModalState {
    #[default]
    Closed,
    SaveAllOpen,
    SaveFileOpen,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct App {
    settings: Settings,
    buffers: Buffers,
    output: String,
    #[serde(skip)]
    save_modal_state: SaveModalState,
    // The action to be completed once the current modal is closed
    #[serde(skip)]
    modal_action: Option<ModalAction>,
    #[serde(skip)]
    ignore_dirty: bool,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_menu_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.visuals_mut().button_frame = false;
                self.menu_bar(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                // TODO: make the explorer fixed width, and full-screen height
                let explorer = Explorer::new(
                    // TODO: the directory should be set when the user uses "Open folder"
                    std::env::current_dir().expect("Unable to get current directory"),
                ).ui(ui);
                
                if let Some(path) = explorer.open_file {
                    self.open_file(Some(path));
                }

                StripBuilder::new(ui)
                    .size(Size::relative(0.5))
                    .size(Size::relative(0.5))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            ui.vertical(|ui| {
                                if let Some(action) =
                                    self.buffers.show(&self.settings, ui).save_modal_action
                                {
                                    self.save_modal_state = SaveModalState::SaveFileOpen;
                                    self.modal_action = Some(action);
                                }
                            });
                        });
                        strip.cell(|ui| {
                            let size = ui.available_size();
                            self.output(ui, size);
                        })
                    });
            });
        });

        match self.save_modal_state {
            SaveModalState::Closed => {
                if let Some(action) = self.modal_action.take() {
                    self.modal_action(action);
                }
            }
            SaveModalState::SaveAllOpen => self.show_save_modal(ctx, true),
            SaveModalState::SaveFileOpen => self.show_save_modal(ctx, false),
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
                ui.separator();
                if ui.button("Save file").clicked() {
                    // TODO: we ignore this for now
                    // in future, this button should be greyed out if no buffer is selected, so we should be able to unwrap here
                    let _ = self.save_file();
                }
                if ui.button("Save as...").clicked() {
                    // TODO: same as above
                    // if the error is `SaveError::NoFileSelected`, we should just ignore this anyway
                    let _ = self.save_as();
                }
                if ui.button("Save all changes").clicked() {
                    self.save_all();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui
                    .add_enabled(
                        true, /* self.text_edit.is_some() */
                        Button::new("Undo"),
                    )
                    .clicked()
                {
                    println!("undo");
                    // TODO: come back to undo/redo buttons
                    // if let Some(text_edit) = self.text_edit.as_mut() {
                    //     println!("actually undo (has undo: {}", text_edit.undoer().has_undo(&(text_edit.cursor.char_range().unwrap_or_default(), self.code_buffer.clone())));
                    //     if let Some((ccursor_range, contents)) = text_edit.undoer().undo(&(text_edit.cursor.char_range().unwrap_or_default(), self.code_buffer.clone())) {
                    //         text_edit.cursor.set_char_range(Some(ccursor_range.clone()));
                    //         self.code_buffer = contents.clone();
                    //     }
                    // }
                }
                if ui
                    .add_enabled(
                        true, /* self.text_edit.is_some() */
                        Button::new("Redo"),
                    )
                    .clicked()
                {
                    log::debug!("redo");
                }
                ui.separator();
                if ui.button("Settings").clicked() {}
            });
            ui.menu_button("View", |_ui| {});
            ui.menu_button("Run", |ui| {
                if ui.button("Run").clicked() {
                    self.run();
                }
            });
            ui.menu_button("Help", |_ui| {});

            // TODO: move this to a settings menu
            ui.checkbox(&mut self.settings.auto_save, "Auto save");

        });
    }

    fn output(&self, ui: &mut egui::Ui, size: egui::Vec2) {
        ui.add_sized(
            size,
            egui::TextEdit::multiline(&mut self.output.as_str())
                .desired_width(f32::INFINITY)
                .code_editor(),
        );
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
            let buffer = self.buffers.get_mut_by_id(id).unwrap();
            if let Err(BufferError::NoAssociatedFile) = buffer.save() {
                self.buffers.select(id);
                // if no file is selected, ignore it and continue saving all
                // TODO: make it so the ui updates which tab is selected inbetween individual calls to `save_as`
                let _ = self.save_as();
            }
        }
    }

    fn open_file(&mut self, path: Option<PathBuf>) {
        // TODO: move this to open folder/project
        // if !self.ignore_dirty && self.is_dirty() {
        //     self.save_modal_state = SaveModalState::SaveAllOpen;
        //     self.modal_action = Some(ModalAction::OpenFile);
        //     return;
        // }

        let Some(path) = path.or_else(|| rfd::FileDialog::new().pick_file()) else {
            log::info!("no file selected to open");
            return;
        };

        // Don't open a new tab if the file is already open
        if let Some(buffer) = self.buffers.get_by_path(&path) {
            self.buffers.select(buffer.id());
            return;
        }

        let buffer = Buffer::from_path(path);

        self.buffers.add(buffer);
    }

    fn modal_action(&mut self, action: ModalAction) {
        println!("modal action {action:?}");
        println!("{self:?}");
        match action {
            ModalAction::OpenFile => self.open_file(None),
            ModalAction::DeleteBuffer(id) => self.buffers.delete_buffer(id),
            ModalAction::Close => todo!(),
        }
        self.ignore_dirty = false;
        self.modal_action = None;
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
                        self.save_modal_state = SaveModalState::Closed;
                    } else {
                        match self.save_file() {
                            Err(SaveError::NoFileSelected) => {}
                            _ => self.save_modal_state = SaveModalState::Closed,
                        }
                    }
                }
                if ui.button("Don't save").clicked() {
                    self.ignore_dirty = true;
                    self.save_modal_state = SaveModalState::Closed;
                }
                if ui.button("Cancel").clicked() {
                    self.modal_action = None;
                    self.save_modal_state = SaveModalState::Closed;
                }
            })
        });
    }

    fn run(&mut self) {
        self.output += "\nHello, world!";
    }
}
