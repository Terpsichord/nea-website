use super::{BackendHandle, Project, ProjectSettings};
use std::sync::{Arc, Mutex};
use ws_messages::{Command, RunAction};

#[derive(Default)]
pub struct Runner {
    handle: BackendHandle,
    is_running: bool,
}

impl Runner {
    pub fn new(handle: BackendHandle) -> Self {
        Self {
            handle,
            is_running: false,
        }
    }

    pub fn run(&mut self, _project: &mut Project, output: Arc<Mutex<String>>) -> eyre::Result<()> {
        output.lock().unwrap().clear();

        self.is_running = true;

        self.handle.send(Command::ReadSettings {
            action: RunAction::Run,
        });
        Ok(())
    }

    pub fn run_action(&mut self, settings: &ProjectSettings, action: RunAction) {
        match action {
            RunAction::Run => self.handle.send(Command::Run {
                command: settings.run_command.to_string(),
            }),
            _ => todo!(),
        }
    }

    pub fn set_finished(&mut self) {
        self.is_running = false;
    }

    pub fn update(&mut self) {
        // TODO
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn stop(&mut self) {
        // TODO
    }
}
