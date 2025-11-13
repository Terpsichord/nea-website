use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware,
    routing::{get, post},
};
use tracing::instrument;

use crate::{
    AppState,
    api::UserResponse,
    auth::middleware::{AuthUser, auth_middleware},
    db::DatabaseConnector,
    error::AppError,
};

pub fn follow_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/follow", get(get_follow_list))
        .route("/follow/{username}", get(get_follow).post(post_follow))
        .route("/follow/{username}/unfollow", post(post_unfollow))
        .route("/followers", get(get_followers))
        .route_layer(middleware::from_fn_with_state(state, auth_middleware))
}

async fn get_follow_list() -> Result<Json<Vec<UserResponse>>, AppError> {
    todo!()
}

/// Checks if the authenticated user currently follows the given user
#[instrument(skip(db))]
async fn get_follow(
    Path(username): Path<String>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
    State(db): State<DatabaseConnector>,
) -> Result<Json<bool>, AppError> {
    let follows = sqlx::query_scalar!(
        r#" 
        SELECT EXISTS (
            SELECT 1 FROM follows f
            INNER JOIN users u1 ON f.follower_id = u1.id
            INNER JOIN users u2 ON f.followee_id = u2.id
            WHERE u1.github_id = $1
            AND u2.username = $2
        ) AS "follows!"
        "#,
        github_id,
        username
    )
    .fetch_one(&*db)
    .await?;

    Ok(Json(follows))
}

#[instrument(skip(db))]
async fn post_follow(
    Path(username): Path<String>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
    State(db): State<DatabaseConnector>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO follows (follower_id, followee_id)
        SELECT
        (SELECT id FROM users WHERE github_id = $1),
        (SELECT id FROM users WHERE username = $2)
        "#,
        github_id,
        username
    )
    .execute(&*db)
    .await?;

    Ok(())
}

#[instrument(skip(db))]
async fn post_unfollow(
    Path(username): Path<String>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
    State(db): State<DatabaseConnector>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        DELETE FROM follows 
        WHERE follower_id = (SELECT id FROM users WHERE github_id = $1)
        AND followee_id = (SELECT id FROM users WHERE username = $2)
        "#,
        github_id,
        username
    )
    .execute(&*db)
    .await?;

    Ok(())
}

#[instrument(skip(db))]
async fn get_followers(
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
    State(db): State<DatabaseConnector>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let followers = sqlx::query_as!(
        UserResponse,
        r#"
        SELECT u.username, u.picture_url, u.bio, u.join_date
        FROM users u
        INNER JOIN follows f ON u.id = f.follower_id
        INNER JOIN users fe ON f.followee_id = fe.id
        WHERE fe.github_id = $1
    "#,
        github_id
    )
    .fetch_all(&*db)
    .await?;

    Ok(Json(followers))
}
