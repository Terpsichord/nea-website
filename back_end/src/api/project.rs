use std::cmp::Reverse;

use axum::{
    Extension, Json, Router,
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, instrument};

use crate::{
    AppState,
    api::ProjectResponse,
    auth::{
        SharedTokenInfo, TokenHeaders,
        middleware::{AuthUser, auth_middleware, optional_auth_middleware},
    },
    db::DatabaseConnector,
    error::AppError,
    github::{CreateRepoResponse, access_tokens::WithTokens},
};

use super::ProjectInfo;

pub fn project_router(state: AppState) -> Router<AppState> {
    let auth = Router::new()
        .route("/project/open/{username}/{repo_name}", get(open_project))
        .route("/project/new", post(new_project))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .route("/projects", get(get_project_list))
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

    Router::new()
        .merge(auth)
        .merge(optional_auth)
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
async fn get_project_list(
    State(db): State<DatabaseConnector>,
) -> Result<Json<Vec<ProjectResponse>>, AppError> {
    let projects = sqlx::query_as!(ProjectResponse, r#"
        SELECT
            (p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags, pi.like_count) as "info!: ProjectInfo",
            pi.github_url as "github_url!",
            p.upload_time,
            p.public,
            false as "owned!"
        FROM projects p
        INNER JOIN project_info pi ON pi.id = p.id
        WHERE p.public
        ORDER BY upload_time DESC
    "#).fetch_all(&*db).await?;

    Ok(Json(projects))
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
    lang: String, // FIXME
    private: bool,
}

#[derive(Serialize)]
struct NewProjectResponse {
    username: String,
    repo_name: String,
}

#[instrument(skip(db, access, refresh))]
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
) -> Result<Json<NewProjectResponse>, AppError> {
    let mut access_token = &*access;
    let mut refresh_token = &*refresh;
    let mut tokens = None;

    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE github_id = $1", github_id)
        .fetch_one(&*db)
        .await?;
    info!("user_id: {}", user_id);

    // check if project with same name exists for user
    let exists = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM projects
            WHERE title = $1
            AND user_id = $2
        ) as "exists!"
        "#,
        title,
        user_id
    )
    .fetch_one(&*db)
    .await?;
    info!("exists: {}", exists);

    if exists {
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
        new_tokens,
    ) = client
        .create_repo(access_token, refresh_token, &username, &title, private)
        .await?;
    info!("repo_name: {}", repo_name);

    tokens = new_tokens;
    if let Some(ref tokens) = tokens {
        (access_token, refresh_token) = tokens.unencrypted();
    }

    let readme = if already_exists {
        let WithTokens(readme, new_tokens) = client
            .get_readme(access_token, refresh_token, &username, &repo_name)
            .await?;
        info!("readme: {}", repo_name);

        tokens = new_tokens;
        if let Some(ref tokens) = tokens {
            (access_token, refresh_token) = tokens.unencrypted();
        }

        Some(readme)
    } else {
        None
    }
    .unwrap_or_default();

    // insert project details into db
    sqlx::query!(
        r#"
        INSERT INTO projects (title, user_id, repo_name, readme, public)
        VALUES ($1, $2, $3, $4, $5)
        "#,
        title,
        user_id,
        repo_name,
        readme,
        !private
    )
    .execute(&*db)
    .await?;

    // FIXME: this function should return the token cookie headers
    // i'm not sure what the return type should be
    Ok(Json(NewProjectResponse {
        username,
        repo_name,
    }))
}

#[instrument(skip(db, session_mgr, access_token, refresh_token))]
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

    session_mgr
        .open(
            project.user_id,
            project.id,
            &username,
            &repo_name,
            &access_token,
            &refresh_token,
        )
        .await?;

    // let code = session_mgr.create_code(project.user_id);

    Ok(ws.on_upgrade(handle_editor_ws))
}

async fn handle_editor_ws(ws: WebSocket) {}

// async fn connect_session(
//     Path((username, repo_name, code)): Path<(String, String, String)>,
//     State(AppState {
//         db, session_mgr, ..
//     }): State<AppState>,
//     ws: WebSocketUpgrade,
// ) -> Result<Response, AppError> {
//     let user_id = sqlx::query_scalar!(
//         r#"
//         SELECT id
//         FROM users
//         WHERE username = $1
//         "#,
//         username
//     )
//     .execute(&*db)
//     .fetch_one()
//     .await?;

//     Ok(ws.on_upgrade(||))
// }
