use std::{fmt::Display, path::PathBuf};

use bincode::config;
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

pub use bincode::error::{DecodeError, EncodeError};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClientMessage {
    pub id: Uuid,
    pub cmd: Command,
}

impl ClientMessage {
    pub fn new(cmd: Command) -> Self {
        Self {
            id: Uuid::new_v4(),
            cmd,
        }
    }
}

impl ClientMessage {
    pub fn encode(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::serde::encode_to_vec(self, config::standard())
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let (msg, _bytes) = bincode::serde::decode_from_slice(encoded, config::standard())?;

        Ok(msg)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RunAction {
    Run,
    Debug,
    Format,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorSettings {
    pub color_scheme: Option<String>,
    pub auto_save: bool,
    pub format_on_save: bool,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            color_scheme: None,
            auto_save: true,
            format_on_save: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    OpenProject,
    ReadSettings { action: RunAction },
    UpdateSettings { settings: EditorSettings },
    ReadFile { path: PathBuf },
    ReadDir { path: PathBuf },
    Rename { from: PathBuf, to: PathBuf },
    WriteFile { path: PathBuf, contents: String },
    Delete { path: PathBuf },
    Run { command: String },
    StopRunning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerMessage {
    pub id: Uuid,
    pub resp: Response,
}

impl ServerMessage {
    pub fn encode(&self) -> Result<Vec<u8>, bincode::error::EncodeError> {
        bincode::serde::encode_to_vec(self, config::standard())
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        let (msg, _bytes) = bincode::serde::decode_from_slice(encoded, config::standard())?;

        Ok(msg)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProjectTree {
    Directory {
        path: PathBuf,
        children: Vec<ProjectTree>,
    },
    File {
        path: PathBuf,
    },
}

impl From<PathBuf> for ProjectTree {
    fn from(path: PathBuf) -> Self {
        if path.to_string_lossy().ends_with("/") {
            ProjectTree::Directory {
                path,
                children: vec![],
            }
        } else {
            ProjectTree::File { path }
        }
    }
}

impl ProjectTree {
    pub fn path(&self) -> &PathBuf {
        match self {
            ProjectTree::Directory { path, .. } => path,
            ProjectTree::File { path } => path,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    Project {
        contents: ProjectTree,
        settings: EditorSettings,
    },
    ProjectSettings {
        contents: String,
    },
    FileContents {
        contents: String,
    },
    DirContents {
        contents_paths: Vec<PathBuf>,
    },
    Output {
        output: String,
    },
    Success,
    Error {
        msg: String,
    },
}

impl<E: Display> From<Result<Response, E>> for Response {
    fn from(res: Result<Response, E>) -> Self {
        match res {
            Ok(resp) => resp,
            Err(err) => Response::Error {
                msg: err.to_string(),
            },
        }
    }
}
