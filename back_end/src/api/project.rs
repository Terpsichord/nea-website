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
