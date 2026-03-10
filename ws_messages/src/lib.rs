use std::{
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
};

use bincode::config;
use ecolor::Color32;
use eyre::eyre;
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
    ColorSchemes,
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
pub struct ColorScheme {
    pub name: String,
    // 16 24-bit colors
    pub bases: [Color32; 16],
}

impl ColorScheme {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn read_from_yaml(reader: impl std::io::Read) -> eyre::Result<Self> {
        let yaml: serde_yaml::Value = serde_yaml::from_reader(reader)?;

        let name = yaml["scheme"]
            .as_str()
            .ok_or(eyre!("invalid color scheme: missing name"))?
            .to_string();

        let mut scheme = Self {
            name,
            bases: [Color32::BLACK; 16],
        };

        for i in 0..16 {
            let name = format!("base0{:X}", i);
            let hex = yaml[name]
                .as_str()
                .ok_or(eyre!("invalid color scheme yaml"))?;

            scheme.bases[i] = Color32::from_hex(&format!("#{hex}"))
                .map_err(|_| eyre!("invalid color in yaml"))?;
        }

        Ok(scheme)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[rustfmt::skip]
pub enum Response {
    Project { contents: ProjectTree, settings: EditorSettings },
    ProjectSettings { contents: String },
    AvailableSchemes { color_schemes: Vec<ColorScheme> },
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
            Err(err) => Response::Error {
                msg: err.to_string(),
            },
        }
    }
}
