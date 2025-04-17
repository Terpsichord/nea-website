use axum::{
    extract::{Path, State},
    middleware,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::{FromRow, PgPool};

use crate::{
    middlewares::auth::{auth_middleware, AuthUser},
    AppState,
};

use super::{user::ProjectInfo, AppError};

pub fn project_route(state: AppState) -> Router<AppState> {
    let auth_router = Router::new()
        .route("/project/{username}/{repo_name}/liked", get(get_liked))
        .route("/project/{username}/{repo_name}/like", post(like))
        .route("/project/{username}/{repo_name}/unlike", post(unlike))
        .layer(middleware::from_fn_with_state(state, auth_middleware));

    Router::new()
        .route("/project/{username}/{repo_name}", get(get_project))
        .route("/projects", get(get_project_list))
        .merge(auth_router)
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct ProjectResponse {
    #[serde(flatten)]
    info: ProjectInfo,
    github_url: String,
    upload_time: NaiveDateTime,
}

async fn get_project(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<PgPool>,
) -> Result<Json<ProjectResponse>, AppError> {
    let project = sqlx::query_as!(ProjectResponse, r#"
        SELECT ROW(p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags, pi.like_count) as "info!: ProjectInfo", pi.github_url as "github_url!", p.upload_time
        FROM projects p
        INNER JOIN project_info pi ON pi.id = p.id
        WHERE pi.username = $1
        AND p.repo_name = $2
    "#, username, repo_name).fetch_one(&db).await?;

    Ok(Json(project))
}


async fn get_project_list(
    State(db): State<PgPool>,
) -> Result<Json<Vec<ProjectResponse>>, AppError> {
    let projects = sqlx::query_as!(ProjectResponse, r#"
        SELECT ROW(p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags, pi.like_count) as "info!: ProjectInfo", pi.github_url as "github_url!", p.upload_time
        FROM projects p
        INNER JOIN project_info pi ON pi.id = p.id
    "#).fetch_all(&db).await?;

    Ok(Json(projects))
}

async fn get_liked(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<PgPool>,
    Extension(AuthUser { github_id }): Extension<AuthUser>,
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
    .fetch_one(&db)
    .await?;

    Ok(Json(liked))
}

async fn like(Path((username, repo_name)): Path<(String, String)>, State(db): State<PgPool>, Extension(AuthUser { github_id }): Extension<AuthUser>) -> Result<(), AppError> {
    sqlx::query!(r#"
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
    ).execute(&db).await?;

    Ok(())
}

async fn unlike(Path((username, repo_name)): Path<(String, String)>, State(db): State<PgPool>, Extension(AuthUser { github_id }): Extension<AuthUser>) -> Result<(), AppError> {
    sqlx::query!(r#"
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
    ).execute(&db).await?;

    Ok(())
}