use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use wasm_bindgen_futures::spawn_local;

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
pub struct Project {
    id: String,
    #[serde(skip)]
    info: Rc<OnceCell<ProjectInfo>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectInfo {
    github_url: String,
}

impl Project {
    pub fn new(id: String) -> Self {
        let info = Rc::new(OnceCell::new());
        {
            let info = info.clone();
            let endpoint = format!("/api/project/open/{id}");
            spawn_local(async move {
                let project_info = Request::get(&endpoint)
                    .send()
                    .await
                    .expect("failed to open project")
                    .json::<ProjectInfo>()
                    .await
                    .expect("failed to parse project info");
                web_sys::console::log_1(&format!("project github_url: {project_info:?}").into());
                info.set(project_info)
                    .expect("failed to update project info");
            });
        }

        Self { id, info }
    }
}
