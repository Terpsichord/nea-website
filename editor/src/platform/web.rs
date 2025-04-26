pub struct Runner;

impl Runner {
    pub fn run(&mut self, project: &mut Project, output: Arc<Mutex<String>>) -> eyre::Result<()> {
        todo!()
    }

    pub fn update(&mut self) {
        todo!()
    }

    pub fn is_running(&self) -> bool {
        todo!()
    }

    pub fn stop(&mut self) {
        todo!()
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Project;

impl Project {
    pub fn new() -> Self {
        todo!()
    }
}