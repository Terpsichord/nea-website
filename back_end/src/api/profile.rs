use axum::{
    Extension, Json, Router,
    extract::State,
    http::{HeaderName, header},
    middleware,
    routing::{delete, get, patch},
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{info, instrument};

use crate::auth::ACCESS_COOKIE;

use crate::{
    AppState,
    api::{ProjectInfo, UserResponse},
    auth::middleware::{AuthUser, auth_middleware},
    db::DatabaseConnector,
    error::AppError,
};

pub fn profile_router(state: AppState) -> Router<AppState> {
    let auth = Router::new()
        .route("/profile", get(get_profile))
        .route("/profile/bio", patch(update_bio))
        .route("/profile/projects", get(get_projects))
        .route("/profile/delete", delete(delete_profile))
        .layer(middleware::from_fn_with_state(state, auth_middleware));

    Router::new()
        .route("/profile/auth", get(auth_handler))
        .route("/profile/signout", post(sign_out))
        .merge(auth)
}

#[instrument(skip(db))]
async fn get_profile(
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
    State(db): State<DatabaseConnector>,
) -> Result<Json<UserResponse>, AppError> {
    let user = sqlx::query_as!(
        UserResponse,
        "SELECT username, picture_url, bio, join_date FROM users WHERE github_id = $1",
        github_id
    )
    .fetch_one(&*db)
    .await?;

    Ok(Json(user))
}

async fn auth_handler(jar: CookieJar) -> Json<Value> {
    Json(json!({ "isAuth": jar.get(ACCESS_COOKIE).is_some() }))
}

#[derive(Deserialize)]
struct UpdateBio {
    bio: String,
}

#[instrument(skip(db))]
async fn update_bio(
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
    State(db): State<DatabaseConnector>,
    Json(UpdateBio { bio }): Json<UpdateBio>,
) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE users SET bio = $1 WHERE github_id = $2",
        bio,
        github_id
    )
    .execute(&*db)
    .await?;

    Ok(())
}

#[instrument(skip(db))]
async fn get_projects(
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
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
        WHERE pi.github_id = $1
        "#,
        github_id
    )
    .fetch_all(&*db)
    .await?;

    Ok(Json(projects))
}


async fn delete_profile(
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
    State(db): State<DatabaseConnector>,
) -> Result<(), AppError> {
    sqlx::query!("DELETE FROM users WHERE github_id = $1", github_id)
        .execute(&*db)
        .await?;

    // we don't need to delete anything else as the user deletion will cascade to the other tables

    Ok(())
}
