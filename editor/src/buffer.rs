use std::{
    fmt::Debug,
    io,
    ops::Deref,
    path::{Path, PathBuf},
};

use crate::{
    app::ModalAction,
    platform::{FileSystem, FileSystemTrait as _},
};
use color_eyre::Section;
use egui::{Response, RichText, ScrollArea, TextEdit, Ui};
use egui_extras::syntax_highlighting::{self, CodeTheme};
use eyre::{Context, eyre};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use ws_messages::EditorSettings;

#[derive(Debug)]
pub struct FileData {
    pub path: PathBuf,
    pub contents: String,
}

pub enum BufferError {
    /// The buffer doesn't have a file associated with it.
    NoAssociatedFile,
    IoError(io::Error),
}

pub struct BuffersOutput {
    pub save_modal_action: Option<ModalAction>,
    pub error_message: Option<String>,
}

#[derive(Debug)]
struct Rename {
    buffer_id: Uuid,
    name: String,
    just_started: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Buffers {
    buffers: Vec<Buffer>,
    selected_id: Option<Uuid>,
    #[serde(skip)]
    rename: Option<Rename>,
}

impl Buffers {
    pub fn show(
        &mut self,
        settings: &EditorSettings,
        ui: &mut Ui,
        code_theme: &CodeTheme,
        fs: &FileSystem,
    ) -> BuffersOutput {
        let (delete_id, renamed) = self.show_tabs(ui);

        if renamed {
            // can unwrap as `renamed` is only set to true if `rename` is Some
            let rename = self.rename.take().unwrap();
            self.get_mut_by_id(rename.buffer_id)
                .and_then(|b| b.rename(&rename.name, fs).ok())
                .expect("failed to rename buffer");
        }

        // show text edit for current buffer
        let mut error_message = None;
        if let Some(buffer) = self.current_buffer_mut() {
            let buffer_view = buffer.show(ui, code_theme);

            if buffer_view.clicked_elsewhere() && settings.auto_save && self.is_dirty() {
                let mut failed_to_save = vec![];
                for buf in self.buffers.iter_mut() {
                    // Ignore `BufferError::NoAssociatedFile` as we ignore buffers that don't have files in auto save
                    if let Err(BufferError::IoError(err)) = buf.save(fs) {
                        failed_to_save.push((err, &*buf));
                    }
                }
                if !failed_to_save.is_empty() {
                    error_message = Some(Self::join_save_errors(failed_to_save));
                }
            }
        } else {
            ui.label("No file open...");
        }

        let mut save_modal_action = None;

        // If there is a buffer to delete
        if let Some(id) = delete_id
            && let Some(buffer) = self.get_by_id(id)
        {
            // If buffer is dirty, then firstly show an "unsaved changes" modal, and then continue with deletion
            // Otherwise, just delete the buffer
            if buffer.is_dirty() {
                save_modal_action = Some(ModalAction::DeleteBuffer(id));
            } else {
                self.delete_buffer(id);
            }
        }

        BuffersOutput {
            save_modal_action,
            error_message,
        }
    }

    fn show_tabs(&mut self, ui: &mut Ui) -> (Option<Uuid>, bool) {
        let mut to_delete = None;
        let mut renamed = false;

        ui.horizontal(|ui| {
            ui.visuals_mut().button_frame = false;
            for buffer in self.buffers.iter() {
                if let Some(rename) = self
                    .rename
                    .as_mut()
                    .filter(|rename| buffer.id == rename.buffer_id)
                {
                    let text_edit = ui.add_sized(
                        [100.0, ui.available_height()],
                        TextEdit::singleline(&mut rename.name),
                    );

                    if rename.just_started {
                        ui.memory_mut(|mem| mem.request_focus(text_edit.id));
                        rename.just_started = false;
                    }

                    if text_edit.lost_focus() {
                        renamed = true;
                    }
                } else {
                    ui.scope(|ui| {
                        if let Some(selected_id) = self.selected_id {
                            if buffer.id == selected_id {
                                ui.visuals_mut().button_frame = true;
                            }
                        }

                        let tab = ui.button(buffer.file_display_name());

                        if tab.clicked() {
                            self.selected_id = Some(buffer.id);
                        }

                        if tab.double_clicked() && buffer.file_data.is_some() {
                            self.rename = Some(Rename {
                                buffer_id: buffer.id,
                                name: buffer
                                    .file_data()
                                    .and_then(|f| f.path.file_name())
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned(),
                                just_started: true,
                            });
                        }
                    });

                    if ui.button("x").clicked() {
                        to_delete = Some(buffer.id);
                    }
                }
                ui.separator();
            }
        });

        (to_delete, renamed)
    }

    fn join_save_errors(errors: Vec<(io::Error, &Buffer)>) -> String {
        let main_message = eyre!(
            "Failed to save file{}: {}",
            if errors.len() > 1 { "s" } else { "" },
            errors
                .iter()
                .map(|(_, buf)| buf.file_display_name().text().to_string())
                .join(", ")
        );
        errors
            .into_iter()
            .fold(Err(main_message), |acc: Result<(), _>, (err, _)| {
                acc.error(err)
            })
            .unwrap_err()
            .to_string()
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

        if self.selected_id.is_some_and(|selected| selected == id) {
            self.selected_id = self.buffers.last().map(|buf| buf.id);
        }
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

    pub fn get_by_path(&self, path: &Path) -> Option<&Buffer> {
        self.buffers
            .iter()
            .find(|buf| buf.file_data.as_ref().map(|f| &*f.path.deref()) == Some(path))
    }

    pub fn is_dirty(&self) -> bool {
        self.buffers.iter().any(|buf| buf.is_dirty())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Buffer {
    id: Uuid,
    contents: String,
    #[serde(skip)]
    // FIXME: this is only currently serde(skip) because i can't serialize pathbuf on wasm
    file_data: Option<FileData>,
}

impl Buffer {
    pub fn new(contents: String, file_data: Option<FileData>) -> Self {
        Self {
            id: Uuid::new_v4(),
            contents,
            file_data,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_path(path: PathBuf, fs: &FileSystem) -> eyre::Result<Self> {
        let contents = fs.read_file(&path).wrap_err("Failed to read file")?;

        Ok(Self::new(
            contents.clone(),
            Some(FileData { contents, path }),
        ))
    }

    pub fn empty() -> Self {
        Self::new(String::new(), None)
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
    pub fn save(&mut self, fs: &FileSystem) -> Result<(), BufferError> {
        let file = self
            .file_data
            .as_mut()
            .ok_or(BufferError::NoAssociatedFile)?;

        let result = fs.write(&file.path, &self.contents);

        #[cfg(not(target_arch = "wasm32"))]
        result.map_err(BufferError::IoError)?;

        file.contents = self.contents.clone();

        Ok(())
    }

    fn rename(&mut self, new_name: &str, fs: &FileSystem) -> Result<(), BufferError> {
        let file = self
            .file_data
            .as_mut()
            .ok_or(BufferError::NoAssociatedFile)?;
        let mut new_path = file.path.clone();

        // TODO: santise the new name and check for errors
        new_path.set_file_name(new_name);

        fs.rename(&file.path, &new_path)
            .map_err(BufferError::IoError)?;

        file.path = new_path;

        Ok(())
    }

    fn show(&mut self, ui: &mut Ui, theme: &CodeTheme) -> Response {
        ScrollArea::vertical()
            .show(ui, |ui| {
                let size = ui.available_size();

                let lang = self
                    .file_data
                    .as_ref()
                    .and_then(|f| f.path.extension())
                    .unwrap_or_default();

                ui.add_sized(
                    size,
                    egui::TextEdit::multiline(&mut self.contents)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .layouter(&mut |ui: &Ui, contents, wrap_width| {
                            let mut layout_job = syntax_highlighting::highlight(
                                ui.ctx(),
                                ui.style(),
                                theme,
                                contents.as_str(),
                                &lang.to_string_lossy(),
                            );
                            layout_job.wrap.max_width = wrap_width;
                            ui.fonts_mut(|f| f.layout_job(layout_job))
                        }),
                )
            })
            .inner
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_save_errors() {
        let buffer1 = Buffer::new(
            "content1".to_string(),
            Some(FileData {
                path: PathBuf::from("file1.txt"),
                contents: "content1".to_string(),
            }),
        );
        let buffer2 = Buffer::new(
            "content2".to_string(),
            Some(FileData {
                path: PathBuf::from("file2.txt"),
                contents: "content2".to_string(),
            }),
        );

        let error1 = io::Error::new(io::ErrorKind::Other, "error1");
        let error2 = io::Error::new(io::ErrorKind::Other, "error2");

        let errors = vec![(error1, &buffer1), (error2, &buffer2)];

        let result = Buffers::join_save_errors(errors);

        eprintln!("Result: {}", result);
        assert!(result.contains("Failed to save files: file1.txt, file2.txt"));
        assert!(result.contains("error1"));
        assert!(result.contains("error2"));
    }
}
