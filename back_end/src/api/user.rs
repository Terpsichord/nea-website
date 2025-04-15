use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use sqlx::{FromRow, PgPool};

use crate::{user::UserResponse, AppState};

use super::AppError;

pub fn user_route() -> Router<AppState> {
    Router::new()
        .route("/user/{username}", get(get_user))
        .route("/user/{username}/projects", get(get_user_projects))
}

async fn get_user(
    Path(username): Path<String>,
    State(db): State<PgPool>,
) -> Result<Json<UserResponse>, AppError> {
    let user = sqlx::query_as!(
        UserResponse,
        "SELECT username, picture_url, bio, join_date FROM users WHERE username = $1",
        username
    )
    .fetch_one(&db)
    .await?;

    Ok(Json(user))
}

#[derive(Serialize, FromRow)]
struct ProjectInfo {
    title: String,
    readme: String,
    tags: Vec<String>,
}

async fn get_user_projects(
    Path(username): Path<String>,
    State(db): State<PgPool>,
) -> Result<Json<Vec<ProjectInfo>>, AppError> {
    let projects = sqlx::query_as!(ProjectInfo, r#"
        SELECT p.title, p.readme, ARRAY_REMOVE(ARRAY_AGG(t.tag), NULL) AS "tags!: Vec<String>"
        FROM projects p
        LEFT JOIN project_tags t ON t.project_id = p.id
        INNER JOIN users u ON p.user_id = u.id
        WHERE u.username = $1
        GROUP BY p.id, p.title, p.readme
    "#, username).fetch_all(&db).await?;

    Ok(Json(projects))
}
