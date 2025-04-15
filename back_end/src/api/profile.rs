use axum::{
    extract::State,
    middleware,
    routing::{get, patch},
    Extension, Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    middlewares::auth::{auth_middleware, AuthUser},
    user::UserResponse,
    AppState,
};

use super::AppError;

pub fn profile_route(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_profile))
        .route("/profile/bio", patch(update_bio))
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
    sqlx::query!("UPDATE users SET bio = $1 WHERE github_id = $2", bio, github_id).execute(&db).await?;

    Ok(())
}
