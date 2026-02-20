use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
    time::Duration,
};

use bollard::{
    Docker, body_full,
    query_parameters::{
        CreateContainerOptions, CreateImageOptionsBuilder, StartContainerOptions,
        StopContainerOptions, UploadToContainerOptions,
    },
    secret::{ContainerCreateBody, HostConfig, Mount, MountTypeEnum},
};
use futures::executor::block_on;
use futures_util::StreamExt as _;
use tokio::task::JoinHandle;
use tracing::{debug, info, instrument, warn};

use crate::{
    error::AppError,
    github::{GithubClient, access_tokens::WithTokens},
    lang::ProjectLang,
};

#[derive(Debug)]
pub struct SessionHandle {
    project_id: i32,
    container_id: String,
    // code: Option<(String, DateTime<Utc>)>,
    // path: String,
}

#[derive(Debug)]
struct WaitingHandle {
    handle: JoinHandle<()>,
}

impl WaitingHandle {
    const DELAY: Duration = Duration::from_secs(30); // TODO: change to 5 minutes

    fn new(session_mgr: EditorSessionManager, user_id: i32) -> Self {
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Self::DELAY).await;
            info!("stopping container for user {}", user_id);
            if let Err(err) = session_mgr.end_session(user_id).await {
                warn!("error when stopping container {err:?}");
            }
        });

        Self { handle }
    }
}

impl std::ops::Drop for WaitingHandle {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[derive(Debug)]
enum SessionMode {
    Active,
    Waiting(WaitingHandle),
}

#[derive(Debug)]
pub struct SessionState {
    handle: SessionHandle,
    mode: SessionMode,
}

// table linking user IDs to state about their current editor session
#[derive(Default, Debug)]
struct SessionTable(HashMap<i32, SessionState>);

// this ensures that all containers are stopped when the program is stopped
impl Drop for SessionTable {
    fn drop(&mut self) {
        if let Ok(docker) = Docker::connect_with_local_defaults() {
            warn!("stopping docker containers on exit");
            for session in self.0.values() {
                let _ = block_on(
                    docker
                        .stop_container(&session.handle.container_id, None::<StopContainerOptions>),
                );
            }
        }
    }
}

impl Deref for SessionTable {
    type Target = HashMap<i32, SessionState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SessionTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug)]
pub struct EditorSessionManager {
    table: Arc<RwLock<SessionTable>>,
    docker: Docker,
    client: GithubClient,
}

impl Default for EditorSessionManager {
    fn default() -> Self {
        Self {
            table: Arc::default(),
            docker: Docker::connect_with_local_defaults().expect("failed to connect to docker"),
            client: GithubClient::default(),
        }
    }
}

impl EditorSessionManager {
    // const CODE_LEN: usize = 16;

    // pub fn create_code(&self, user_id: i32) -> String {
    //     let code = BASE64_STANDARD.encode(rand::random::<[u8; Self::CODE_LEN]>());
    //     let expiry = Utc::now() + Duration::minutes(5);

    //     self.table.write().unwrap().entry(user_id).and_modify(|state| state.handle.code = Some((code.clone(), expiry)));

    //     code
    // }

    #[allow(clippy::too_many_arguments)] // FIXME
    pub async fn open(
        &self,
        user_id: i32,
        project_id: i32,
        username: &str,
        repo_name: &str,
        lang: ProjectLang,
        access_token: &str,
        refresh_token: &str,
    ) -> Result<WithTokens<String>, AppError> {
        let mut reactivate = false;
        let mut close = false;

        if let Some(state) = self.table.read().unwrap().get(&user_id) {
            match state.mode {
                SessionMode::Active => return Err(AppError::SessionConflict),
                SessionMode::Waiting(_) => {
                    if state.handle.project_id == project_id {
                        reactivate = true;
                    } else {
                        close = true;
                    }
                }
            }
        }

        if reactivate {
            self.table
                .write()
                .unwrap()
                .entry(user_id)
                .and_modify(|state| state.mode = SessionMode::Active);
            return Ok(WithTokens::default());
        }

        if close {
            self.end_session(user_id).await?;
        }

        info!("creating session");

        self.create_session(
            user_id,
            project_id,
            username,
            repo_name,
            lang,
            access_token,
            refresh_token,
        )
        .await
    }

    pub const WORKSPACE_PATH: &'static str = "/home/workspace";

    #[instrument(skip(self, access_token, refresh_token))]
    #[allow(clippy::too_many_arguments)] // FIXME
    async fn create_session(
        &self,
        user_id: i32,
        project_id: i32,
        username: &str,
        repo_name: &str,
        lang: ProjectLang,
        access_token: &str,
        refresh_token: &str,
    ) -> Result<WithTokens<String>, AppError> {
        let image = self.get_image(lang).await?;

        let mount = Mount {
            target: Some(Self::WORKSPACE_PATH.into()),
            source: None,
            typ: Some(MountTypeEnum::VOLUME),
            read_only: Some(false),
            ..Default::default()
        };

        let config = ContainerCreateBody {
            image: Some(image),
            // enable tty to allow an interactive terminal on the frontend
            tty: Some(true),
            host_config: Some(HostConfig {
                // runsc is the runtime needed to use gVisor
                runtime: Some("runsc".into()),
                auto_remove: Some(true),
                mounts: Some(vec![mount]),
                ..Default::default()
            }),
            ..Default::default()
        };

        debug!("creating container");
        let container_id = self
            .docker
            .create_container(None::<CreateContainerOptions>, config)
            .await
            .map_err(AppError::other)?
            .id;

        debug!("starting container: {container_id}");
        self.docker
            .start_container(&container_id, None::<StartContainerOptions>)
            .await
            .map_err(AppError::other)?;

        debug!("fetching files");
        let WithTokens(tarball, headers) = self
            .client
            .get_project_tarball(access_token, refresh_token, username, repo_name)
            .await?;

        debug!("adding files to container");
        self.docker
            .upload_to_container(
                &container_id,
                Some(UploadToContainerOptions {
                    path: Self::WORKSPACE_PATH.into(),
                    ..Default::default()
                }),
                body_full(tarball),
            )
            .await
            .map_err(AppError::other)?;

        self.table.write().unwrap().insert(
            user_id,
            SessionState {
                handle: SessionHandle {
                    project_id,
                    container_id: container_id.clone(),
                },
                mode: SessionMode::Active,
            },
        );

        Ok(WithTokens(container_id, headers))
    }

    async fn get_image(&self, lang: ProjectLang) -> Result<String, AppError> {
        // TODO: change this to actually get the right image, depending on the language used by the project
        let image = "python:3".to_string();

        // this ensures that the image is present on the host system
        // TODO: probably get rid of this if i end up using custom dockerfile images
        self.docker
            .create_image(
                Some(
                    CreateImageOptionsBuilder::default()
                        .from_image(&image)
                        .build(),
                ),
                None,
                None,
            )
            .next()
            .await
            .unwrap()
            .map_err(AppError::other)?;

        Ok(image)
    }

    pub async fn idle_session(&self, user_id: i32) {
        self.table
            .write()
            .unwrap()
            .entry(user_id)
            .and_modify(|state| {
                state.mode = SessionMode::Waiting(WaitingHandle::new(self.clone(), user_id))
            });
    }

    pub async fn end_session(&self, user_id: i32) -> Result<(), AppError> {
        let maybe_session = self.table.write().unwrap().remove(&user_id);
        if let Some(session) = maybe_session {
            self.docker
                .stop_container(&session.handle.container_id, None::<StopContainerOptions>)
                .await
                .map_err(AppError::other)?;
        }

        Ok(())
    }
}
