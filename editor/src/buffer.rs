use std::{fmt::Debug, path::PathBuf};

use crate::app::{ModalAction, Settings};
use egui::{Response, RichText, Ui, Widget};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileData {
    pub path: PathBuf,
    pub contents: String,
}

pub enum BufferError {
    /// The buffer doesn't have a file associated with it.
    NoAssociatedFile,
}

pub struct BuffersOutput {
    pub save_modal_action: Option<ModalAction>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Buffers {
    buffers: Vec<Buffer>,
    selected_id: Option<Uuid>,
}

impl Buffers {
    pub fn show(&mut self, settings: &Settings, ui: &mut Ui) -> BuffersOutput {
        let mut to_delete = None;

        ui.horizontal(|ui| {
            ui.visuals_mut().button_frame = false;
            for buffer in self.buffers.iter() {
                ui.scope(|ui| {
                    if let Some(selected_id) = self.selected_id {
                        if buffer.id == selected_id {
                            ui.visuals_mut().button_frame = true;
                        }
                    }

                    if ui.button(buffer.file_display_name()).clicked() {
                        self.selected_id = Some(buffer.id);
                    }
                });

                if ui.button("x").clicked() {
                    to_delete = Some(buffer.id);
                }
                ui.separator();
            }
        });

        if let Some(buffer) = self.current_buffer_mut() {
            if ui.add(&mut *buffer).clicked_elsewhere() && settings.auto_save && self.is_dirty() {
                for buf in self.buffers.iter_mut() {
                    // Ignore buffers that don't have files in auto save
                    let _ = buf.save();
                }
            }
        } else {
            ui.label("No file open...");
        }

        let save_modal_action = to_delete.and_then(|to_delete| {
            // If buffer is dirty, then firstly show an "unsaved changes" modal, and then continue with deletion
            // Otherwise, just delete the buffer
            let save_modal_action = if let Some(deleted_buffer) = self.get_by_id(to_delete) {
                if deleted_buffer.is_dirty() {
                    Some(ModalAction::DeleteBuffer(deleted_buffer.id))
                } else {
                    self.delete_buffer(deleted_buffer.id);
                    None
                }
            } else {
                None
            };

            // If selected tab is closed, select last tab (if any are open)
            if self.selected_id.is_some_and(|id| id == to_delete) {
                self.selected_id = self.buffers.last().map(|buf| buf.id);
            }

            save_modal_action
        });

        BuffersOutput { save_modal_action }
    }

    pub fn add(&mut self, buffer: Buffer) {
        self.select(buffer.id);
        self.buffers.push(buffer);
    }

    pub fn select(&mut self, id: Uuid) {
        self.selected_id = Some(id);
    }

    pub fn delete_buffer(&mut self, id: Uuid) {
        self.buffers.retain(|buffer| id != buffer.id);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Buffer> {
        self.buffers.iter()
    }

    pub fn get_by_id(&self, id: Uuid) -> Option<&Buffer> {
        self.buffers.iter().find(|buf| buf.id == id)
    }

    pub fn get_mut_by_id(&mut self, id: Uuid) -> Option<&mut Buffer> {
        self.buffers.iter_mut().find(|buf| buf.id == id)
    }

    pub fn current_buffer(&self) -> Option<&Buffer> {
        self.selected_id.and_then(|id| self.get_by_id(id))
    }

    pub fn current_buffer_mut(&mut self) -> Option<&mut Buffer> {
        self.selected_id.and_then(|id| self.get_mut_by_id(id))
    }

    pub fn get_by_path(&self, path: &PathBuf) -> Option<&Buffer> {
        self.buffers
            .iter()
            .find(|buf| buf.file_data.as_ref().map(|f| &f.path) == Some(path))
    }

    pub fn is_dirty(&self) -> bool {
        self.buffers.iter().any(|buf| buf.is_dirty())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Buffer {
    id: Uuid,
    contents: String,
    pub file_data: Option<FileData>,
}

impl Buffer {
    pub fn new(contents: String, file_data: Option<FileData>) -> Self {
        Self {
            id: Uuid::new_v4(),
            contents,
            file_data,
        }
    }

    pub fn from_path(path: PathBuf) -> Self {
        let contents = std::fs::read_to_string(&path).expect("Unable to read file");

        Self {
            id: Uuid::new_v4(),
            contents: contents.clone(),
            file_data: Some(FileData { contents, path }),
        }
    }

    pub fn empty() -> Self {
        Self {
            id: Uuid::new_v4(),
            contents: String::new(),
            file_data: None,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    pub fn file_data(&self) -> Option<&FileData> {
        self.file_data.as_ref()
    }

    pub fn set_file_data(&mut self, file_data: FileData) {
        self.file_data = Some(file_data);
    }

    fn file_display_name(&self) -> RichText {
        self.file_data
            .as_ref()
            .and_then(|f| {
                let dirty = self.is_dirty();
                let text = f.path.file_name()?.to_string_lossy().to_string();
                Some(if dirty {
                    RichText::new(text + " *").italics()
                } else {
                    text.into()
                })
            })
            .unwrap_or(
                RichText::new("Untitled".to_string() + if self.is_dirty() { " *" } else { "" })
                    .italics(),
            )
    }

    // TODO: rewrite doc comment
    /// Checks whether the current buffer is dirty.
    ///
    /// If a file is associated with the `App`, this returns `true` if the
    /// contents of `self.contents` differ from `self.file_data.contents`.
    /// Otherwise, it returns `true` if the buffer isn't empty.
    pub fn is_dirty(&self) -> bool {
        match &self.file_data {
            Some(f) => self.contents != f.contents,
            None => !self.contents.trim().is_empty(),
        }
    }

    // returns an error if the buffer has no associated file
    pub fn save(&mut self) -> Result<(), BufferError> {
        let file = self
            .file_data
            .as_mut()
            .ok_or(BufferError::NoAssociatedFile)?;

        std::fs::write(&file.path, &self.contents).expect("Unable to write file");
        file.contents = self.contents.clone();

        Ok(())
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::empty()
    }
}

impl Widget for &mut Buffer {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = ui.available_size();

        ui.add_sized(
            size,
            egui::TextEdit::multiline(&mut self.contents)
                .code_editor()
                .desired_width(f32::INFINITY),
        )
    }
}
