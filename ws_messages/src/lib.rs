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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    OpenProject,
    ReadFile { path: PathBuf },
    ReadDir { path: PathBuf },
    Rename { from: PathBuf, to: PathBuf },
    WriteFile { path: PathBuf, contents: String },
    Delete { path: PathBuf },
    Run,
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
    ProjectContents { contents: ProjectTree },
    FileContents { contents: String },
    DirContents { contents_paths: Vec<PathBuf> },
    Output { output: String },
    Success,
    Error { msg: String },
}

impl<E: Display> From<Result<Response, E>> for Response {
    fn from(res: Result<Response, E>) -> Self {
        match res {
            Ok(resp) => resp,
            Err(err) => Response::Error { msg: err.to_string() },
        }
    }
}