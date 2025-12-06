use super::{BackendHandle, Project};
use std::sync::{Arc, Mutex};
use ws_messages::Command;

#[derive(Default)]
pub struct Runner {
    handle: BackendHandle,
}

impl Runner {
    pub fn new(handle: BackendHandle) -> Self {
        Self { handle }
    }

    pub fn run(&mut self, _project: &mut Project, _output: Arc<Mutex<String>>) -> eyre::Result<()> {
        self.handle.send(Command::ReadFile {
            path: "src/main.rs".into(),
        });
        Ok(())
    }

    pub fn update(&mut self) {
        // TODO
    }

    pub fn is_running(&self) -> bool {
        // TODO
        false
    }

    pub fn stop(&mut self) {
        // TODO
    }
}
