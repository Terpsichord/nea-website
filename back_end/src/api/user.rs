use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use tracing::instrument;

use crate::{
    AppState,
    api::{ProjectInfo, UserResponse},
    db::DatabaseConnector,
    error::AppError,
};

pub fn user_router() -> Router<AppState> {
    Router::new()
        .route("/user/{username}", get(get_user))
        .route("/user/{username}/projects", get(get_user_projects))
}

#[instrument(skip(db))]
async fn get_user(
    Path(username): Path<String>,
    State(db): State<DatabaseConnector>,
) -> Result<Json<UserResponse>, AppError> {
    let user = sqlx::query_as!(
        UserResponse,
        "SELECT username, picture_url, bio, join_date FROM users WHERE username = $1",
        username
    )
    .fetch_one(&*db)
    .await?;

    Ok(Json(user))
}

#[instrument(skip(db))]
async fn get_user_projects(
    Path(username): Path<String>,
    State(db): State<DatabaseConnector>,
) -> Result<Json<Vec<ProjectInfo>>, AppError> {
    let projects = sqlx::query_as!(
        ProjectInfo,
        r#"
        SELECT 
            p.title,
            pi.username as "username!",
            pi.picture_url as "picture_url!",
            p.repo_name,
            p.readme,
            pi.tags as "tags!",
            pi.like_count as "like_count!"
        FROM projects p
        INNER JOIN project_info pi ON p.id = pi.id
        WHERE pi.username = $1
        AND p.public
        "#,
        username
    )
    .fetch_all(&*db)
    .await?;

    Ok(Json(projects))
}
