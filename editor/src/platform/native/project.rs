use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{ProjectSettings, ProjectSettingsError};

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub(super) path: PathBuf,
    pub(super) settings: Option<ProjectSettings>,
}

impl Project {
    pub fn new(path: PathBuf, settings: Option<ProjectSettings>) -> Self {
        Self { path, settings }
    }
}

impl ProjectSettings {
    pub fn read_from(path: &Path) -> Result<Option<ProjectSettings>, ProjectSettingsError> {
        let full_path = path.join(Self::PATH);

        let contents = match std::fs::read_to_string(full_path) {
            Ok(contents) => contents,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            err => err?,
        };

        let settings = toml::from_str(&contents)?;

        Ok(settings)
    }
}
