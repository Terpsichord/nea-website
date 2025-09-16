use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub run_command: String,
}

#[derive(Debug, Error)]
pub enum ProjectSettingsError {
    #[error("Failed to read project.toml")]
    Io(#[from] io::Error),
    #[error("project.toml has invalid format")]
    Format(#[from] toml::de::Error),
}

// TODO: implement this trait for the web and native structs
pub trait Runner {
    // TODO: should this just require &ProjectSettings (maybe with &mut Project in update() instead?)
    fn run(&mut self, project: &mut Project, output: Arc<Mutex<String>>) -> eyre::Result<()>;
    fn stop(&mut self);
    fn update(&mut self);
    fn is_running(&self) -> bool;
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(target_arch = "wasm32")]
pub use web::*;
