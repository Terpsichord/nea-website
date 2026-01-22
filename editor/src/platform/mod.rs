use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{
    io,
    sync::{Arc, Mutex},
};
use thiserror::Error;

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub run_command: String,
    // TODO: add format_command: Option<String> and debug_command: Option<String>
}

impl ProjectSettings {
    const PATH: &str = ".ide/project.toml";
}

#[derive(Debug, Error)]
pub enum ProjectSettingsError {
    #[error("Failed to read project.toml")]
    Io(#[from] io::Error),
    #[error("project.toml has invalid format")]
    Format(#[from] toml::de::Error),
}

// TODO: implement this trait for the web and native structs
pub trait RunnerTrait {
    // TODO: should this just require &ProjectSettings (maybe with &mut Project in update() instead?)
    // actually maybe for the best to just leave it how it is now
    fn run(&mut self, project: &mut Project, output: Arc<Mutex<String>>) -> eyre::Result<()>;
    fn stop(&mut self);
    fn update(&mut self);
    fn is_running(&self) -> bool;
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line: usize,
    pub col: usize,
}

pub trait FileSystemTrait {
    type ReadDir: Iterator<Item = io::Result<PathBuf>>;

    // fn new_file(&self, path: &Path) -> io::Result<()>;
    fn read_file(&self, path: &Path) -> io::Result<String>;
    fn read_dir(&self, path: &Path) -> io::Result<Self::ReadDir>;
    fn search_project(&self, project: &Project, pattern: &str) -> Vec<SearchResult>;
    fn replace(&self, paths: &[&Path], pattern: &str, replace: &str) -> io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn write(&self, path: &Path, contents: &str) -> io::Result<()>;
    fn delete(&self, path: &Path) -> io::Result<()>;
}
