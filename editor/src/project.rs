use std::{io::{self, ErrorKind}, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub path: PathBuf,
    pub settings: Option<ProjectSettings>
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub run_command: String,
}

impl ProjectSettings {
    const PATH: &str = ".myide/project.toml";

    pub fn read_from(path: &Path) -> Result<Option<ProjectSettings>, ProjectSettingsError> {
        let y = path.join(Self::PATH);
        let contents = match std::fs::read_to_string(y) {
            Ok(contents) => contents,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            err => err?,
        };
        let x = toml::from_str(&contents)?;
        Ok(x)
    }
}

#[derive(Debug, Error)]
pub enum ProjectSettingsError {
    #[error("Failed to read project.toml")]
    Io(#[from] io::Error),
    #[error("project.toml has invalid format")]
    Format(#[from] toml::de::Error),
}

