use std::path::PathBuf;

use bincode::config;
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
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
    pub fn decode(encoded: &[u8]) -> Result<Self, bincode::error::DecodeError> {
        bincode::serde::decode_from_slice(encoded, config::standard()).map(|(out, _)| out)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    FileContents { contents: String },
    DirContents { contents_paths: Vec<PathBuf> },
    Output { output: String },
    Success,
}
