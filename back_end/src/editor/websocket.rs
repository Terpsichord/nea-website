use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use axum::extract::ws::{Message, WebSocket};
use bollard::{
    Docker,
    exec::{CreateExecOptions, StartExecOptions, StartExecResults},
};
use futures::{Stream, StreamExt as _, stream};
use serde::Serialize;
use tokio::sync::Mutex;
use tracing::{info, warn};
use ws_messages::{ClientMessage, Command, ProjectTree, ServerMessage};

pub struct WebSocketHandler {
    docker: Docker,
    container_id: String,
}

fn assert_send<T: Send>(_: &T) {}

impl WebSocketHandler {
    pub fn new(container_id: String) -> anyhow::Result<Self> {
        Ok(Self {
            docker: Docker::connect_with_local_defaults()?,
            container_id,
        })
    }

    // TODO: error handling
    pub async fn handle(&self, ws: WebSocket) {
        let ws = Arc::new(Mutex::new(ws));
        while let Some(recv) = ws.lock().await.recv().await {
            match recv {
                Ok(Message::Binary(msg)) => {
                    let msgs = match self.get_response(&msg).await {
                        Ok(msgs) => msgs,
                        Err(err) => {
                            warn!("failed to execute command on websocket: {}", err);
                            continue;
                        }
                    };

                    msgs.for_each(|msg| {
                        let ws = ws.clone();
                        async move {
                            let fut = ws.lock();
                            assert_send(&fut);
                            let _ = fut
                                .await
                                .send(Message::Binary(msg.encode().expect("TODO").into()))
                                .await;
                        }
                    })
                    .await;
                }
                Ok(Message::Text(_)) => warn!("received text on websocket"),
                Ok(Message::Close(_)) => {} // TODO
                Ok(_) => {}
                Err(err) => warn!("failed to receive message on websocket: {}", err),
            }
        }
    }

    async fn get_response(
        &self,
        msg: &[u8],
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = ServerMessage> + '_>>> {
        let ClientMessage { id, cmd } = ClientMessage::decode(msg)?;

        let msgs = Box::pin(self.execute_cmd(cmd).await?.map({
            let id = id.clone();
            move |resp| ServerMessage { id, resp }
        }));

        Ok(msgs)
    }

    async fn execute_cmd(
        &self,
        cmd: Command,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = ws_messages::Response> + '_>>> {
        info!("executing command: {:?}", cmd);
        Ok(match cmd {
            Command::OpenProject => Box::pin(self.open_project().await?),
            Command::Run => Box::pin(self.run().await?),
            Command::ReadFile { path } => Box::pin(self.read_file(path).await?),
            _ => todo!(),
        })
    }

    async fn exec_docker<T>(
        &self,
        cmd: Vec<T>,
    ) -> Result<impl Stream<Item = String>, bollard::errors::Error>
    where
        T: Into<String> + Default + Serialize,
    {
        let msg = self
            .docker
            .create_exec(
                &self.container_id,
                CreateExecOptions {
                    cmd: Some(cmd),
                    attach_stdout: Some(true),
                    ..Default::default()
                },
            )
            .await?;

        let StartExecResults::Attached { output, .. } =
            self.docker.start_exec(&msg.id, None).await?
        else {
            unreachable!()
        };

        let stream = output
            .filter_map(|res| async move { res.ok() })
            .map(|o| o.to_string());

        Ok(stream)
    }

    async fn open_project(
        &self,
    ) -> Result<impl Stream<Item = ws_messages::Response>, bollard::errors::Error> {
        Ok(stream::once(async move {
            ws_messages::Response::ProjectContents {
                contents: ProjectTree::Directory {
                    path: "/".into(),
                    children: vec![],
                },
            }
        }))
    }

    async fn run(
        &self,
    ) -> Result<impl Stream<Item = ws_messages::Response>, bollard::errors::Error> {
        let run_cmd = "echo 'hello world'"; // TODO: somehow get a run cmd

        Ok(self
            .exec_docker(vec!["sh", "-c", run_cmd])
            .await?
            .map(|output| ws_messages::Response::Output { output }))
    }

    async fn read_file(
        &self,
        path: PathBuf,
    ) -> Result<impl Stream<Item = ws_messages::Response>, bollard::errors::Error> {
        let out: Vec<String> = self
            .exec_docker(vec!["cat".into(), path.to_string_lossy()])
            .await?
            .collect()
            .await;

        Ok(stream::once(async move {
            ws_messages::Response::FileContents {
                contents: out.join("\n"),
            }
        }))
    }
}
