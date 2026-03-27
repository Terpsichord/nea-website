use std::{
    io,
    path::{Path, PathBuf},
};

use async_tar::Archive;
use axum::extract::ws::{Message, WebSocket};
use base64::{Engine as _, prelude::BASE64_STANDARD};
use bollard::{
    exec::{CreateExecOptions, StartExecResults},
    secret::ExecInspectResponse,
};
use futures::{AsyncReadExt as _, StreamExt as _, TryStreamExt as _};
use serde::Serialize;
use tokio::io::AsyncWriteExt as _;
use tokio_util::{compat::TokioAsyncReadCompatExt, io::StreamReader};
use tracing::{info, warn};
use ws_messages::{ClientMessage, Command, EditorSettings, ProjectTree, Response, ServerMessage};

use crate::{DatabaseConnector, auth::crypto::Aes256Gcm, editor::session::EditorSessionManager};

// Class that handles incoming WebSocket messages for a single user session
pub struct WebSocketHandler {
    db: DatabaseConnector,
    session_mgr: EditorSessionManager,
    container_id: String,
    user_id: i32,
    running_pid: Option<i64>,
    project_dir: Option<String>,
}

impl WebSocketHandler {
    pub const fn new(
        container_id: String,
        user_id: i32,
        db: DatabaseConnector,
        session_mgr: EditorSessionManager,
    ) -> Self {
        Self {
            db,
            session_mgr,
            container_id,
            user_id,
            running_pid: None,
            project_dir: None,
        }
    }

    pub async fn handle(&mut self, mut ws: WebSocket) {
        // for each message received...
        while let Some(recv) = ws.recv().await {
            match recv {
                // if binary message received (as expected), try to execute the command and return a response message
                Ok(Message::Binary(msg)) => {
                    let msg = match self.create_response(&msg).await {
                        Ok(msg) => msg,
                        Err(err) => {
                            eprintln!("failed to execute command on websocket: {err:#?}");
                            continue;
                        }
                    };

                    let _ = ws
                        .send(Message::Binary(
                            msg.encode()
                                .expect("failed to the encode the ws message")
                                .into(),
                        ))
                        .await;
                }
                // plaintext messages should not be received over the websocket
                Ok(Message::Text(_)) => warn!("received text on websocket"),
                // set the container to waiting when the websocket is closed (e.g. when the browser tab is closed)
                Ok(Message::Close(_)) => {
                    info!("idling container {:?}", &self.container_id);

                    self.session_mgr.idle_session(self.user_id);
                }
                Ok(_) => {}
                Err(err) => warn!("failed to receive message on websocket: {}", err),
            }
        }
    }

    async fn create_response(&mut self, msg: &[u8]) -> anyhow::Result<ServerMessage> {
        let ClientMessage { id, cmd } = ClientMessage::decode(msg)?;

        let resp = self.execute_cmd(cmd).await?;

        Ok(ServerMessage { id, resp })
    }

    #[rustfmt::skip]
    async fn execute_cmd(&mut self, cmd: Command) -> anyhow::Result<Response> {
        println!("executing command: {cmd:?}");
        Ok(match cmd {
            Command::OpenProject                    => self.open_project().await?,
            Command::UpdateSettings { settings }    => self.update_settings(settings).await?,
            Command::ReadSettings { .. }            => self.read_settings().await?,
            Command::ColorSchemes                   => self.color_schemes().await?,
            Command::Run { command }                => self.run(&command).await?,
            Command::ReadFile { path }              => self.read_file(&path).await?,
            Command::ReadDir { path }               => self.read_dir(&path).await?,
            Command::WriteFile { path, contents }   => self.write_file(&path, &contents).await?,
            Command::Format { command }             => self.format(&command).await?,
            Command::Rename { from, to }            => self.rename(&from, &to).await?,
            Command::Delete { path }                => self.delete(&path).await?,
            Command::StopRunning                    => self.stop_running().await?,
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
            .session_mgr
            .docker()
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
            self.session_mgr.docker().start_exec(&msg.id, None).await?
        else {
            unreachable!()
        };

        let ExecInspectResponse { pid, .. } =
            self.session_mgr.docker().inspect_exec(&msg.id).await?;

        if let Some(stdin) = stdin {
            input.write_all(stdin.as_bytes()).await?;
            input.shutdown().await?;
        }

        let lines: Vec<String> = output.map_ok(|o| o.to_string()).try_collect().await?;

        Ok((lines.join("\n"), pid.unwrap()))
    }

    async fn open_project(&mut self) -> anyhow::Result<Response> {
        let output = self
            .exec_docker(vec![
                "ls",
                "--indicator-style=slash",
                EditorSessionManager::WORKSPACE_PATH,
            ])
            .await?;
        println!("output: {output}");

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
        println!("top_dir: {:?}", top_dir);

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

        println!("top_dir: {:?}", top_dir);

        self.project_dir = Some(project_dir.to_string());

        Ok(Response::Project {
            contents: ProjectTree::Directory {
                path: format!("{}/{}/", EditorSessionManager::WORKSPACE_PATH, project_dir).into(),
                children: top_dir,
            },
            settings: self.db.get_editor_settings(self.user_id).await?,
        })
    }

    async fn update_settings(&self, settings: EditorSettings) -> Result<Response, sqlx::Error> {
        self.db
            .update_editor_settings(self.user_id, settings)
            .await?;

        Ok(Response::Success)
    }

    async fn read_settings(&self) -> Result<Response, bollard::errors::Error> {
        let (contents, _) = self
            .exec_docker_with(vec!["cat", ".ide/project.toml"], None, true)
            .await?;

        Ok(Response::ProjectSettings { contents })
    }

    async fn color_schemes(&self) -> anyhow::Result<Response> {
        let color_schemes = self.db.get_color_schemes().await?;

        Ok(Response::AvailableSchemes { color_schemes })
    }

    async fn run(&mut self, cmd: &str) -> Result<Response, bollard::errors::Error> {
        let (output, pid) = self
            .exec_docker_with(vec!["sh", "-c", cmd], None, true)
            .await?;

        self.running_pid = Some(pid);

        Ok(Response::Output { output })
    }

    async fn read_file(&self, path: &Path) -> Result<Response, bollard::errors::Error> {
        let contents = self
            .exec_docker(vec!["cat", &path.to_string_lossy()])
            .await?;

        Ok(Response::FileContents { contents })
    }

    async fn read_dir(&self, path: &Path) -> Result<Response, bollard::errors::Error> {
        let contents = self
            .exec_docker(vec![
                "ls",
                "--indicator-style=slash",
                &path.to_string_lossy(),
            ])
            .await?;

        let contents_paths = contents.lines().map(PathBuf::from).collect();

        Ok(Response::DirContents { contents_paths })
    }

    async fn write_file(
        &self,
        path: &Path,
        contents: &str,
    ) -> Result<Response, bollard::errors::Error> {
        self.exec_docker_with(
            vec!["tee", &path.to_string_lossy()],
            Some(contents.to_string()),
            false,
        )
        .await
        .map(|_| Response::Success)
    }

    async fn format(&self, command: &str) -> Result<Response, bollard::errors::Error> {
        self.exec_docker_with(vec!["sh", "-c", command], None, true)
            .await?;

        Ok(Response::Success)
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<Response, bollard::errors::Error> {
        self.exec_docker(vec!["mv", &from.to_string_lossy(), &to.to_string_lossy()])
            .await
            .map(|_| Response::Success)
    }

    async fn delete(&self, path: &Path) -> Result<Response, bollard::errors::Error> {
        // check if path is directory
        if self
            .exec_docker(vec!["ls", "-l", &path.to_string_lossy()])
            .await?
            .lines()
            .next()
            .map(|line| line.starts_with("total"))
            .unwrap_or_default()
        {
            // remove dir
            self.exec_docker(vec!["rm", "-rf", &path.to_string_lossy()])
                .await
                .map(|_| Response::Success)
        } else {
            // remove file
            self.exec_docker(vec!["rm", &path.to_string_lossy()])
                .await
                .map(|_| Response::Success)
        }
    }

    async fn stop_running(&self) -> Result<Response, bollard::errors::Error> {
        if let Some(pid) = self.running_pid {
            self.exec_docker(vec!["kill", &pid.to_string()])
                .await
                .map(|_| Response::Success)
        } else {
            Ok(Response::Success)
        }
    }
}
