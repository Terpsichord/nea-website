use std::{fs, path::PathBuf};

use anyhow::anyhow;
use axum::{
    Extension, Json, Router,
    extract::{Path, State, WebSocketUpgrade, ws::WebSocket},
    middleware,
    response::{IntoResponse as _, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use tempdir::TempDir;
use tokio::process::Command;
use tracing::{info, instrument};
use walkdir::WalkDir;

use crate::{
    AppState,
    api::{ProjectResponse, search},
    auth::{ResponseTokenExt, TokenHeaders, middleware::{AuthUser, auth_middleware, optional_auth_middleware}},
    db::{DatabaseConnector, NewProject},
    editor::{session::EditorSessionManager, websocket::WebSocketHandler},
    error::AppError,
    github::{CreateRepoResponse, access_tokens::{WithTokens, update_tokens}},
    lang::ProjectLang,
};

pub fn project_router(state: AppState) -> Router<AppState> {
    let auth = Router::new()
        // FIXME: also anything with {username}/{repo} should be in project_page router (not sure about update_project - double check this)
        .route("/project/{username}/{repo_name}/open", get(open_project)) // FIXME: fuck this should be post probably (definitely not get)
        .route("/project/new", post(new_project))
        .route("/project/{username}/{repo_name}/remix", post(remix_project))
        .route("/project/{username}/{repo_name}", put(update_project))
        .route("/project/github_save", post(github_save_project))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .route("/project/search", get(search::search_projects))
        .merge(auth)
        .nest(
            "/project/{username}/{repo_name}",
            project_page_router(state),
        )
}

fn project_page_router(state: AppState) -> Router<AppState> {
    let auth = Router::new()
        .route("/liked", get(get_liked))
        .route("/like", post(like))
        .route("/unlike", post(unlike))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let optional_auth =
        Router::new()
            .route("/", get(get_project))
            .layer(middleware::from_fn_with_state(
                state,
                optional_auth_middleware,
            ));

    Router::new().merge(auth).merge(optional_auth)
}

#[instrument(skip(db))]
async fn get_project(
    Path((username, repo_name)): Path<(String, String)>,
    Extension(auth_user): Extension<Option<AuthUser>>,
    State(db): State<DatabaseConnector>,
) -> Result<Json<ProjectResponse>, AppError> {
    let project = db
        .get_project(
            &username,
            &repo_name,
            auth_user.map(|user| user.github_id),
            false,
        )
        .await?;

    Ok(Json(project.into()))
}

#[instrument(skip(db))]
async fn get_liked(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<DatabaseConnector>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
) -> Result<Json<bool>, AppError> {
    let liked = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM likes l
            INNER JOIN projects p ON p.id = l.project_id
            INNER JOIN users lu ON lu.id = l.user_id
            INNER JOIN users pu ON pu.id = p.user_id
            WHERE lu.github_id = $1
            AND pu.username = $2
            AND p.repo_name = $3
        ) as "liked!"
        "#,
        github_id,
        username,
        repo_name
    )
    .fetch_one(&*db)
    .await?;

    Ok(Json(liked))
}

async fn like(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<DatabaseConnector>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO likes (user_id, project_id)
        SELECT 
        (SELECT id FROM users WHERE github_id = $1),
        (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $2
            AND p.repo_name = $3
        )
        "#,
        github_id,
        username,
        repo_name
    )
    .execute(&*db)
    .await?;

    Ok(())
}

#[instrument(skip(db))]
async fn unlike(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<DatabaseConnector>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        DELETE FROM likes
        WHERE user_id = (SELECT id FROM users WHERE github_id = $1)
        AND project_id = 
        (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $2
            AND p.repo_name = $3
        )
        "#,
        github_id,
        username,
        repo_name
    )
    .execute(&*db)
    .await?;

    Ok(())
}

#[derive(Deserialize)]
struct NewProjectBody {
    title: String,
    lang: ProjectLang,
    private: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewProjectResponse {
    username: String,
    repo_name: String,
}

// creating a new project
#[instrument(skip(db, client, access, refresh))]
async fn new_project(
    State(AppState { db, client, .. }): State<AppState>,
    Extension(AuthUser {
        github_id,
        access_token: access,
        refresh_token: refresh,
    }): Extension<AuthUser>,
    Json(NewProjectBody {
        title,
        lang,
        private,
    }): Json<NewProjectBody>,
) -> Result<Response, AppError> {
    let mut access_token = &*access;
    let mut refresh_token = &*refresh;

    let user_id = db.get_user_id(github_id).await?;
    if db.project_exists(user_id, &title).await? {
        return Err(AppError::ProjectExists);
    }

    let username = sqlx::query_scalar!("SELECT username FROM users WHERE id = $1", user_id)
        .fetch_one(&*db)
        .await?;

    // create the github repo for the project
    let WithTokens(
        CreateRepoResponse {
            repo_name,
            already_exists,
        },
        mut tokens,
    ) = client
        .create_repo(
            access_token,
            refresh_token,
            &username,
            &title,
            lang,
            private,
        )
        .await?;
    update_tokens!(access_token, refresh_token, tokens);

    // if the repo already exists, get the readme
    let readme = if already_exists {
        let WithTokens(readme, new_tokens) = client
            .get_readme(access_token, refresh_token, &username, &repo_name)
            .await?;

        tokens = new_tokens.or(tokens);

        Some(readme)
    } else {
        None
    }
    .unwrap_or_default();

    let new_project = NewProject {
        title,
        lang,
        user_id,
        repo_name,
        readme,
        public: !private,
        tags: vec![],
    };

    // add a record to the database for the new project
    db.add_project(&new_project).await?;

    Ok(Json(NewProjectResponse {
        username,
        repo_name: new_project.repo_name,
    }).into_response().with_tokens(tokens))
}

#[instrument(skip(db, client, access_token, refresh_token))]
async fn remix_project(
    Path((username, repo_name)): Path<(String, String)>,
    State(AppState { db, client, .. }): State<AppState>,
    Extension(AuthUser {
        github_id,
        access_token,
        refresh_token,
    }): Extension<AuthUser>,
) -> Result<Json<NewProjectResponse>, AppError> {
    info!("remixing");
    let project = db.get_project(&username, &repo_name, None, false).await?;
    info!("found project {}", project.info.title);

    let user_id = db.get_user_id(github_id).await?;
    info!("user_id: {}", user_id);
    
    if db.project_exists(user_id, &project.info.title).await? {
        return Err(AppError::ProjectExists);
    }

    // create a fork of the github repo for the project
    let WithTokens((), tokens) = client
        .fork_repo(&access_token, &refresh_token, &username, &repo_name)
        .await?;
    info!("repo_name: {}", repo_name);

    let tags = sqlx::query_scalar!(
        r#"
        SELECT tag
        FROM project_tags
        WHERE project_id = $1
        "#,
        project.id
    )
    .fetch_all(&*db)
    .await?;

    let new_project = NewProject {
        title: project.info.title,
        repo_name,
        lang: project.lang,
        user_id,
        readme: project.info.readme,
        public: true,
        tags,
    };

    db.add_project(&new_project).await?;

    let new_username = sqlx::query_scalar!("SELECT username FROM users WHERE id = $1", user_id)
        .fetch_one(&*db)
        .await?;

    Ok(Json(NewProjectResponse {
        username: new_username,
        repo_name: new_project.repo_name,
    }))
}

#[derive(Deserialize)]
struct UpdateProjectBody {
    title: String,
    private: bool,
    tags: Vec<String>,
}

async fn update_project(
    Path((username, repo_name)): Path<(String, String)>,
    State(AppState { db, .. }): State<AppState>,
    Json(UpdateProjectBody {
        title,
        private,
        tags,
    }): Json<UpdateProjectBody>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE projects
        SET title = $1, public = $2
        WHERE id = 
        (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $3
            AND p.repo_name = $4
        )
        "#,
        title,
        !private,
        username,
        repo_name,
    )
    .execute(&*db)
    .await?;

    // remove all old tags
    sqlx::query!(
        r#"
        DELETE FROM project_tags
        WHERE project_id = 
        (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $1
            AND p.repo_name = $2
        )
        "#,
        username,
        repo_name
    )
    .execute(&*db)
    .await?;

    // and add new tags
    sqlx::query!(
        r#"
        INSERT INTO project_tags (project_id, tag)
        SELECT p.id, UNNEST($1::text[])
        FROM projects p
        INNER JOIN users u ON u.id = p.user_id
        WHERE u.username = $2
        AND p.repo_name = $3
        "#,
        &tags,
        username,
        repo_name,
    )
    .execute(&*db)
    .await?;

    Ok(())
}

#[instrument(skip(db, ws, session_mgr, access_token, refresh_token))]
async fn open_project(
    Path((username, repo_name)): Path<(String, String)>,
    State(AppState {
        db, session_mgr, ..
    }): State<AppState>,
    ws: WebSocketUpgrade,
    Extension(AuthUser {
        github_id,
        access_token,
        refresh_token,
    }): Extension<AuthUser>,
) -> Result<Response, AppError> {
    let project = db
        .get_project(&username, &repo_name, Some(github_id), true)
        .await?;

    let WithTokens(container_id, tokens) = session_mgr
        .open(
            project.user_id,
            project.id,
            &username,
            &repo_name,
            project.lang,
            &access_token,
            &refresh_token,
        )
        .await?;

    Ok(ws.on_upgrade(move |ws| {
        handle_editor_ws(
            ws,
            db.clone(),
            session_mgr.clone(),
            container_id,
            project.user_id,
        )
    }).with_tokens(tokens))
}

async fn handle_editor_ws(
    ws: WebSocket,
    db: DatabaseConnector,
    session_mgr: EditorSessionManager,
    container_id: String,
    user_id: i32,
) {
    let mut handler = WebSocketHandler::new(container_id, user_id, db, session_mgr);

    handler.handle(ws).await;
}

async fn github_save_project(
    State(AppState {
        db, session_mgr, ..
    }): State<AppState>,
    Extension(AuthUser {
        access_token,
        refresh_token,
        github_id,
    }): Extension<AuthUser>,
) -> Result<(), AppError> {
    println!("saving project to github");
    let user_id = db.get_user_id(github_id).await?;
    let session_handle = session_mgr.get_active_session(user_id).unwrap();

    let temp_dir = TempDir::new("ide-export").map_err(AppError::other)?;

    let container_path = PathBuf::from(EditorSessionManager::WORKSPACE_PATH).join(&session_handle.directory);
    let status = Command::new("docker")
        .args(["cp", &format!("{}:{}", session_handle.container_id, container_path.to_string_lossy()), "."])
        .current_dir(&temp_dir)
        .status()
        .await
        .map_err(AppError::other)?;

    if !status.success() {
        return Err(AppError::other(anyhow!("failed to export project")));
    }

    let fs_path = temp_dir.path().to_path_buf().join(&session_handle.directory);

    println!("reading files");
    let files = WalkDir::new(&temp_dir);
        
    let mut file_data = vec![];
    for file in files {
        let file = file.map_err(AppError::other)?;
        if !file.file_type().is_file() {
            continue;
        }

        println!("reading file: {file:?}, path: {:?}", file.path());
        println!("trying to strip with prefix: {}", &*fs_path.to_string_lossy());
        let path = file.path().to_string_lossy().to_string().strip_prefix(&*fs_path.to_string_lossy()).expect("invalid path").to_string();

        let contents = fs::read_to_string(file.path()).map_err(AppError::other)?;

        file_data.push((path, contents));
    }

    let project = sqlx::query!(
        r#"
        SELECT pi.username, p.repo_name
        FROM projects p
        INNER JOIN project_info pi ON p.id = pi.id
        WHERE p.id = $1
        "#,
        session_handle.project_id
    )
    .fetch_one(&*db)
    .await?;

    println!("adding files");
    let _ = session_mgr.client().add_multiple_files(
        &access_token,
        &refresh_token,
        &project.username.unwrap(),
        &project.repo_name,
        file_data,
    ).await?;

    Ok(())
}
