use std::io;
use serde::{Serialize, Deserialize};
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

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(target_arch = "wasm32")]
pub use web::*;
