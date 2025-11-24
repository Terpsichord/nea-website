use super::{BackendHandle, Project};
use std::sync::{Arc, Mutex};
use ws_messages::Command;

#[derive(Default)]
pub struct Runner {
    handle: BackendHandle,
}

impl Runner {
    pub fn run(&mut self, _project: &mut Project, _output: Arc<Mutex<String>>) -> eyre::Result<()> {
        self.handle.send(Command::Run);
        Ok(())
    }

    pub fn update(&mut self) {
        todo!()
    }

    pub fn is_running(&self) -> bool {
        // TODO
        false
    }

    pub fn stop(&mut self) {
        todo!()
    }
}
