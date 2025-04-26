use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Runner;

impl Runner {
    pub fn run(&mut self, _project: &mut Project, _output: Arc<Mutex<String>>) -> eyre::Result<()> {
        // make request to some /run api endpoint (or use websockets to send request)
        todo!()
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Project;

impl Project {
    pub fn new() -> Self {
        // TODO
        Self
    }
}
