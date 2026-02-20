use super::{BackendHandle, ProjectSettings, ProjectSettingsError};
use eyre::WrapErr as _;
use serde::{Deserialize, Serialize};
use web_sys::WebSocket;
use ws_messages::Command;

// #[derive(Debug, Serialize, Deserialize)]
// FIXME
#[derive(Debug)]
pub struct Project {
    username: String,
    repo_name: String,
    handle: BackendHandle,
    settings: Option<ProjectSettings>,
}

// TODO: i think remove this unless its used anywhere
#[derive(Debug, Serialize, Deserialize)]
struct ProjectInfo {
    github_url: String,
}

impl Project {
    pub async fn new(username: String, repo_name: String) -> eyre::Result<Self> {
        web_sys::console::log_1(&format!("opening project {username}/{repo_name}").into());
        let endpoint = format!("/api/project/{username}/{repo_name}/open");
        let handle = BackendHandle::new(&endpoint)
            .await
            .wrap_err("failed to create websocket")?;

        handle.send(Command::OpenProject);

        Ok(Self {
            username,
            repo_name,
            handle,
            settings: None,
        })
    }

    pub fn handle(&self) -> &BackendHandle {
        &self.handle
    }

    pub fn set_settings(&mut self, settings: ProjectSettings) {
        self.settings = Some(settings);
    }
}

impl ProjectSettings {
    pub fn from_contents(contents: &str) -> Result<ProjectSettings, ProjectSettingsError> {
        Ok(toml::from_str(&contents)?)
    }
}
