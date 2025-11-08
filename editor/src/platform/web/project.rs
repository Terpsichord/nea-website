use super::BackendHandle;
use serde::{Deserialize, Serialize};
use web_sys::WebSocket;
use eyre::WrapErr as _;

// #[derive(Debug, Serialize, Deserialize)]
// FIXME
#[derive(Debug)]
pub struct Project {
    username: String,
    repo_name: String,
    handle: BackendHandle,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectInfo {
    github_url: String,
}

impl Project {
    pub fn new(username: String, repo_name: String) -> eyre::Result<Self> {
        let endpoint = format!("/api/project/open/{username}/{repo_name}");
        let handle = BackendHandle::new(&endpoint).wrap_err("failed to create websocket")?;

        Ok(Self {
            username,
            repo_name,
            handle,
        })
    }

    pub fn handle(&self) -> &BackendHandle {
        &self.handle
    }
}
