use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::anyhow;
use bollard::{
    Docker, body_full,
    query_parameters::{
        CreateContainerOptions, CreateImageOptionsBuilder, StartContainerOptions,
        StopContainerOptions, UploadToContainerOptions,
    },
    secret::{ContainerCreateBody, HostConfig, Mount, MountTypeEnum},
};
use bytes::Bytes;
use flate2::read::GzDecoder;
use futures::executor::block_on;
use futures_util::StreamExt as _;
use tokio::task::JoinHandle;
use tracing::{debug, info, instrument, warn};

use crate::{
    error::AppError,
    github::{GithubClient, access_tokens::WithTokens},
    lang::ProjectLang,
};

// `SessionHandle` stores needed information about a
// session in the online editor for a single user
#[derive(Clone, Debug)]
pub struct SessionHandle {
    pub project_id: i32,
    pub container_id: String,
    pub directory: String,
    // code: Option<(String, DateTime<Utc>)>,
    // path: String,
}

// Implements wait logic as described in 'Container Lifecycle'   
#[derive(Debug)]
struct WaitingHandle {
    handle: JoinHandle<()>,
}

impl WaitingHandle {
    const DELAY: Duration = Duration::from_mins(5);

    // Starts a new task when constructed which stops the container running after DELAY, unless aborted (see below) 
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

// Uses RAII (Resource Acquisition Is Initialization) pattern to abort the task that stops the Docker container
// This means that when the `WaitingHandle` object is dropped (i.e. goes out of scope), the task is automatically aborted
// This occurs when the `SessionMode` is changed from being `Waiting` to `Active`
impl std::ops::Drop for WaitingHandle {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

// enum to indicate whether an entry in the session table is active or idle
#[derive(Debug)]
#[allow(dead_code)]
enum SessionMode {
    Active,
    Waiting(WaitingHandle),
}


// Encapsulates entire state of a session, aggregating both the handle and current mode
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

// Class that manages all of the sessions in the online editor
// Stores the table of sessions, and holds references to the Docker and GitHub API clients
#[derive(Clone, Debug)]
pub struct EditorSessionManager {
    table: Arc<RwLock<SessionTable>>,
    docker: Docker,
    client: GithubClient,
}

impl Default for EditorSessionManager {
    // Constructor to initalise the manager to default settings
    fn default() -> Self {
        Self {
            table: Arc::default(),
            docker: Docker::connect_with_local_defaults().expect("failed to connect to docker"),
            client: GithubClient::default(),
        }
    }
}

impl EditorSessionManager {
    pub const fn docker(&self) -> &Docker {
        &self.docker
    }

    pub const fn client(&self) -> &GithubClient {
        &self.client
    }

    // FIXME: remove(?) probably
    // const CODE_LEN: usize = 16;

    // pub fn create_code(&self, user_id: i32) -> String {
    //     let code = BASE64_STANDARD.encode(rand::random::<[u8; Self::CODE_LEN]>());
    //     let expiry = Utc::now() + Duration::minutes(5);

    //     self.table.write().unwrap().entry(user_id).and_modify(|state| state.handle.code = Some((code.clone(), expiry)));

    //     code
    // }

    // called when the server receives a HTTP request to open a new session
    // may either start a new container or re-activate an already running one, depending on the state of SessionTable 
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
        // does an already running container need to be reactivated?
        let mut reactivate = false;
        // does an already running container need to closed? 
        let mut close = false;

        // If there is already a running container for the user:
        if let Some(state) = self.table.read().unwrap().get(&user_id) {
            match state.mode {
                // If the container is for an active session the user currently has open,
                // return an error as only one session can be open per user at a time
                SessionMode::Active => return Err(AppError::SessionConflict),
                // If the container is for a session that is waiting to be re-opened...
                SessionMode::Waiting(_) => {
                    // ... and the open request was for the same project...
                    if state.handle.project_id == project_id {
                        // ... then re-activate the session
                        reactivate = true;
                    } else {
                        // otherwise close the container (a new one will be started below)
                        close = true;
                    }
                }
            }
        }

        if reactivate {
            // update the table to set the session to active and return the container id          
            let mut table = self.table.write().unwrap();

            table
                .entry(user_id)
                .and_modify(|state| state.mode = SessionMode::Active);

            let container_id = table.get(&user_id).unwrap().handle.container_id.clone();

            return Ok(WithTokens(container_id, None));
        }

        if close {
            // end the session if needed
            self.end_session(user_id).await?;
        }

        info!("creating session");

        // create a new session (only called if `reactivate` was false)
        let res = self.create_session(
            user_id,
            project_id,
            username,
            repo_name,
            lang,
            access_token,
            refresh_token,
        )
        .await;
    
        if let Err(ref e) = res {
            println!("failed to create session: {e:?}");
        }

        res
    }

    pub const WORKSPACE_PATH: &'static str = "/home/workspace";

    // create a new container via the Docker API and update the session table
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

        // defines configuration for an anonymous mount 
        // this describes how Docker will store the filesystem of the container
        let mount = Mount {
            target: Some(Self::WORKSPACE_PATH.into()),
            // no source as the volume is anonymous
            source: None,
            typ: Some(MountTypeEnum::VOLUME),
            // the volume needs to be writable to let users edit files
            read_only: Some(false),
            ..Default::default()
        };

        // init container config
        let config = ContainerCreateBody {
            image: Some(image),
            // enable tty to allow an interactive terminal on the frontend
            tty: Some(true),
            host_config: Some(HostConfig {
                // ensures that the root fs cannot be modified
                readonly_rootfs: Some(true),
                network_mode: Some("none".into()),
                // runsc is the runtime needed to use gVisor
                runtime: Some("runsc".into()),
                // docker container is automatically destroyed when stopped
                auto_remove: Some(true),
                mounts: Some(vec![mount]),
                ..Default::default()
            }),
            ..Default::default()
        };

        // creating and starting the container
        
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

        // retrieve the files for the project from the GitHub repository
        debug!("fetching files");
        let WithTokens((dir_name, tarball), headers) = self
            .client
            .get_project_tarball(access_token, refresh_token, username, repo_name)
            .await?;

        // add the files to the container
        debug!("adding files to container");
        self.docker
            .upload_to_container(
                &container_id,
                Some(UploadToContainerOptions {
                    path: Self::WORKSPACE_PATH.into(),
                    ..Default::default()
                }),
                body_full(tarball.clone()),
            )
            .await
            .map_err(AppError::other)?;

        // updating session table to add a new session
        self.table.write().unwrap().insert(
            user_id,
            SessionState {
                handle: SessionHandle {
                    project_id,
                    container_id: container_id.clone(),
                    directory: format!("{dir_name}/"),
                },
                mode: SessionMode::Active,
            },
        );

        Ok(WithTokens(container_id, headers))
    }

    // pub fn get_tarball_dir(tar_gz: &Bytes) -> Result<String, AppError> {
    //     let tarball = GzDecoder::new(tar_gz.as_ref());
    //     let mut archive = tar::Archive::new(tarball);
    //     let mut entries = archive.entries().map_err(AppError::other)?;
    //     let entry = entries
    //         .next()
    //         .ok_or(AppError::Other(anyhow!("no entry in tarball")))?
    //         .map_err(AppError::other)?;

    //     let path = entry.path().map_err(AppError::other)?;

    //     path
    //         .components()
    //         .next()
    //         .ok_or(AppError::Other(anyhow!("no dir in tarball")))?
    //         .as_os_str()
    //         .to_str()
    //         .ok_or(AppError::Other(anyhow!("invalid dir name")))
    //         .map(String::from)
    // }


    async fn get_image(&self, lang: ProjectLang) -> Result<String, AppError> {
        // FIXME: change this to actually get the right image, depending on the language used by the project
        // you should build the image from the languages dockerfile first, as such: https://users.rust-lang.org/t/docker-image-not-being-build-with-bollard/129631/3
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

    // update a session to have mode = SessionMode::Waiting
    pub fn idle_session(&self, user_id: i32) {
        self.table
            .write()
            .unwrap()
            .entry(user_id)
            .and_modify(|state| {
                state.mode = SessionMode::Waiting(WaitingHandle::new(self.clone(), user_id));
            });
    }

    // stop the docker container (this also removes the container as auto_remove = true)
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

    pub fn get_active_session(&self, user_id: i32) -> Option<SessionHandle> {
        let table = self.table.read().unwrap();
        let maybe_session = table.get(&user_id);

        if let Some(SessionState {
            handle,
            mode: SessionMode::Active,
        }) = maybe_session
        {
            Some(handle.clone())
        } else {
            None
        }
    }
}
