use axum::{
    extract::State,
    middleware,
    routing::{get, patch},
    Extension, Json, Router,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    middlewares::auth::{auth_middleware, AuthUser, SharedTokenIds},
    user::{auth_user_id, UserResponse},
    AppState,
};

use super::AppError;

pub fn profile_route() -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_profile))
        .route("/profile/bio", patch(update_bio))
        .layer(middleware::from_fn(auth_middleware))
}

async fn get_profile(
    Extension(token_ids): Extension<SharedTokenIds>,
    Extension(user): Extension<AuthUser>,
    State(db): State<PgPool>,
    State(client): State<reqwest::Client>,
) -> Result<Json<UserResponse>, AppError> {
    let id = auth_user_id(&user, &client, &token_ids).await?;

    let user = sqlx::query_as!(
        UserResponse,
        "SELECT username, picture_url, bio, join_date FROM users WHERE github_id = $1",
        id 
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
    Extension(user): Extension<AuthUser>,
    Extension(token_ids): Extension<SharedTokenIds>,
    State(db): State<PgPool>,
    State(client): State<reqwest::Client>,
    Json(UpdateBio { bio }): Json<UpdateBio>,
) -> Result<(), AppError> {
    let id = auth_user_id(&user, &client, &token_ids).await?;
    sqlx::query!("UPDATE users SET bio = $1 WHERE github_id = $2", bio, id).execute(&db).await?;

    Ok(())
}
