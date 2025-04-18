use axum::{
    extract::State,
    middleware,
    routing::{get, patch},
    Extension, Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    api::user::ProjectInfo,
    middlewares::auth::{auth_middleware, AuthUser},
    user::UserResponse,
    AppState,
};

use super::AppError;

pub fn profile_route(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_profile))
        .route("/profile/bio", patch(update_bio))
        .route("/profile/projects", get(get_projects))
        .layer(middleware::from_fn_with_state(state, auth_middleware))
}

async fn get_profile(
    Extension(AuthUser { github_id }): Extension<AuthUser>,
    State(db): State<PgPool>,
) -> Result<Json<UserResponse>, AppError> {
    let user = sqlx::query_as!(
        UserResponse,
        "SELECT username, picture_url, bio, join_date FROM users WHERE github_id = $1",
        github_id
    )
    .fetch_one(&db)
    .await?;

    Ok(Json(user))
}

#[derive(Deserialize)]
struct UpdateBio {
    bio: String,
}

async fn update_bio(
    Extension(AuthUser { github_id }): Extension<AuthUser>,
    State(db): State<PgPool>,
    Json(UpdateBio { bio }): Json<UpdateBio>,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE users SET bio = $1 WHERE github_id = $2",
        bio,
        github_id
    )
    .execute(&db)
    .await?;

    Ok(())
}

async fn get_projects(
    Extension(AuthUser { github_id }): Extension<AuthUser>,
    State(db): State<PgPool>,
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
        WHERE pi.github_id = $1
        "#,
        github_id
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(projects))
}
