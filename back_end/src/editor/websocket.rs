use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use axum::extract::ws::{Message, WebSocket};
use bollard::{
    Docker,
    exec::{CreateExecOptions, StartExecResults},
    secret::ExecInspectResponse,
};
use futures::TryStreamExt as _;
use serde::Serialize;
use tokio::io::AsyncWriteExt as _;
use tracing::{info, warn};
use ws_messages::{ClientMessage, Command, EditorSettings, ProjectTree, ServerMessage};

use crate::{DatabaseConnector, editor::session::EditorSessionManager};

pub struct WebSocketHandler {
    docker: Docker,
    db: DatabaseConnector,
    session_mgr: EditorSessionManager,
    container_id: String,
    user_id: i32,
    running_pid: Option<i64>,
    project_dir: Option<String>,
}

impl WebSocketHandler {
    pub fn new(
        container_id: String,
        user_id: i32,
        db: DatabaseConnector,
        session_mgr: EditorSessionManager,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            docker: Docker::connect_with_local_defaults()?,
            db,
            session_mgr,
            container_id,
            user_id,
            running_pid: None,
            project_dir: None,
        })
    }

    // TODO: error handling
    pub async fn handle(&mut self, mut ws: WebSocket) {
        while let Some(recv) = ws.recv().await {
            match recv {
                Ok(Message::Binary(msg)) => {
                    let msg = match self.create_response(&msg).await {
                        Ok(msg) => msg,
                        Err(err) => {
                            warn!("failed to execute command on websocket: {}", err);
                            continue;
                        }
                    };

                    let _ = ws
                        .send(Message::Binary(msg.encode().expect("TODO").into()))
                        .await;
                }
                Ok(Message::Text(_)) => warn!("received text on websocket"),
                Ok(Message::Close(_)) => {} // TODO
                Ok(_) => {}
                Err(err) => warn!("failed to receive message on websocket: {}", err),
            }
        }

        info!("idling container {:?}", &self.container_id);

        self.session_mgr.idle_session(self.user_id).await; 
    }

    async fn create_response(&mut self, msg: &[u8]) -> anyhow::Result<ServerMessage> {
        let ClientMessage { id, cmd } = ClientMessage::decode(msg)?;

        let resp = self.execute_cmd(cmd).await?;

        Ok(ServerMessage { id, resp })
    }

    #[rustfmt::skip]
    async fn execute_cmd(&mut self, cmd: Command) -> anyhow::Result<ws_messages::Response> {
        info!("executing command: {:?}", cmd);
        Ok(match cmd {
            Command::OpenProject                  => self.open_project().await?,
            Command::UpdateSettings { settings }  => self.update_settings(settings).await?,
            Command::ReadSettings { action }      => self.read_settings().await?,
            Command::Run { command }              => self.run(&command).await?,
            Command::ReadFile { path }            => self.read_file(&path).await?,
            Command::ReadDir { path }             => self.read_dir(&path).await?,
            Command::WriteFile { path, contents } => self.write_file(&path, &contents).await?,
            _ => todo!(),
        })
    }

    async fn exec_docker<T>(&self, cmd: Vec<T>) -> Result<String, bollard::errors::Error>
    where
        T: Into<String> + Default + Serialize,
    {
        self.exec_docker_with(cmd, None, false)
            .await
            .map(|(output, _)| output)
    }

    async fn exec_docker_with<T>(
        &self,
        cmd: Vec<T>,
        stdin: Option<String>,
        use_working_dir: bool,
    ) -> Result<(String, i64), bollard::errors::Error>
    where
        T: Into<String> + Default + Serialize,
    {
        let cmd = cmd.into_iter().map(Into::into).collect();
        let msg = self
            .docker
            .create_exec(
                &self.container_id,
                CreateExecOptions::<String> {
                    cmd: Some(cmd),
                    working_dir: use_working_dir.then(|| {
                        format!(
                            "{}/{}",
                            EditorSessionManager::WORKSPACE_PATH,
                            self.project_dir.as_ref().map_or("", |p| p),
                        )
                    }),
                    attach_stdin: Some(true),
                    attach_stdout: Some(true),
                    ..Default::default()
                },
            )
            .await?;

        let StartExecResults::Attached { mut input, output } =
            self.docker.start_exec(&msg.id, None).await?
        else {
            unreachable!()
        };

        let ExecInspectResponse { pid, .. } = self.docker.inspect_exec(&msg.id).await?;

        if let Some(stdin) = stdin {
            input.write_all(stdin.as_bytes()).await?;
            input.shutdown().await?;
        }

        let lines: Vec<String> = output.map_ok(|o| o.to_string()).try_collect().await?;

        Ok((lines.join("\n"), pid.unwrap()))
    }

    async fn open_project(&mut self) -> anyhow::Result<ws_messages::Response> {
        let output = self
            .exec_docker(vec![
                "ls",
                "--indicator-style=slash",
                EditorSessionManager::WORKSPACE_PATH,
            ])
            .await?;

        let project_dir = output.lines().next().expect("missing project dir");

        let mut top_dir: Vec<ProjectTree> = self
            .exec_docker(vec![
                "ls",
                &format!("{}/{}", EditorSessionManager::WORKSPACE_PATH, project_dir),
            ])
            .await?
            .lines()
            .map(|entry| {
                PathBuf::from(format!(
                    "{}/{}/{}",
                    EditorSessionManager::WORKSPACE_PATH,
                    project_dir,
                    entry
                ))
                .into()
            })
            .collect();

        if self
            .exec_docker(vec![
                "ls",
                "--indicator-style=slash",
                "-a",
                &format!("{}/{}", EditorSessionManager::WORKSPACE_PATH, project_dir),
            ])
            .await?
            .lines()
            .any(|x| x == ".ide/")
        {
            top_dir.push(
                PathBuf::from(format!(
                    "{}/{}/.ide/",
                    EditorSessionManager::WORKSPACE_PATH,
                    project_dir
                ))
                .into(),
            );
        }

        info!("top_dir: {:?}", top_dir);

        self.project_dir = Some(project_dir.to_string());

        Ok(ws_messages::Response::Project {
            contents: ProjectTree::Directory {
                path: format!("{}/{}/", EditorSessionManager::WORKSPACE_PATH, project_dir).into(),
                children: top_dir,
            },
            settings: self.db.get_editor_settings(self.user_id).await?,
        })
    }

    async fn update_settings(
        &self,
        settings: EditorSettings,
    ) -> Result<ws_messages::Response, sqlx::Error> {
        self.db
            .update_editor_settings(self.user_id, settings)
            .await?;

        Ok(ws_messages::Response::Success)
    }

    async fn read_settings(&self) -> Result<ws_messages::Response, bollard::errors::Error> {
        let (contents, _) = self
            .exec_docker_with(vec!["cat", ".ide/project.toml"], None, true)
            .await?;

        Ok(ws_messages::Response::ProjectSettings { contents })
    }

    async fn run(&mut self, cmd: &str) -> Result<ws_messages::Response, bollard::errors::Error> {
        let (output, pid) = self
            .exec_docker_with(vec!["sh", "-c", cmd], None, true)
            .await?;

        self.running_pid = Some(pid);

        Ok(ws_messages::Response::Output { output })
    }

    async fn read_file(
        &self,
        path: &Path,
    ) -> Result<ws_messages::Response, bollard::errors::Error> {
        let contents = self
            .exec_docker(vec!["cat", &path.to_string_lossy()])
            .await?;

        Ok(ws_messages::Response::FileContents { contents })
    }

    async fn read_dir(&self, path: &Path) -> Result<ws_messages::Response, bollard::errors::Error> {
        let contents = self
            .exec_docker(vec![
                "ls",
                "--indicator-style=slash",
                &path.to_string_lossy(),
            ])
            .await?;

        let contents_paths = contents.lines().map(PathBuf::from).collect();

        Ok(ws_messages::Response::DirContents { contents_paths })
    }

    async fn write_file(
        &self,
        path: &Path,
        contents: &str,
    ) -> Result<ws_messages::Response, bollard::errors::Error> {
        self.exec_docker_with(
            vec!["tee", &path.to_string_lossy()],
            Some(contents.to_string()),
            false,
        )
        .await
        .map(|_| ws_messages::Response::Success)
    }
}
