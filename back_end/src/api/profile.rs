use axum::{
    extract::State,
    middleware,
    routing::{get, patch},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize, Serializer};
use sqlx::MySqlPool;
use time::{format_description, Date};

use crate::{
    middlewares::auth::{auth_middleware, AuthUser, SharedTokenIds},
    user::auth_user_id,
    AppState,
};

use super::AppError;

pub fn user_route() -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_user))
        .route("/profile/bio", patch(update_bio))
        .layer(middleware::from_fn(auth_middleware))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UserResponse {
    username: String,
    picture_url: String,
    bio: String,
    #[serde(serialize_with = "serialize_join_date")]
    join_date: Date,
}

// ref is required by serde
#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_join_date<S: Serializer>(join_date: &Date, serializer: S) -> Result<S::Ok, S::Error> {
    let format = format_description::parse("[year]-[month]-[day]").unwrap();
    join_date.format(&format).expect("failed to format join_date").serialize(serializer)

}

async fn get_user(
    Extension(token_ids): Extension<SharedTokenIds>,
    Extension(user): Extension<AuthUser>,
    State(db): State<MySqlPool>,
    State(client): State<reqwest::Client>,
) -> Result<Json<UserResponse>, AppError> {
    let id = auth_user_id(&user, &client, &token_ids).await?;

    let user = sqlx::query_as!(
        UserResponse,
        "SELECT username, picture_url, bio, join_date FROM users WHERE github_id = ?",
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
    State(db): State<MySqlPool>,
    State(client): State<reqwest::Client>,
    Json(UpdateBio { bio }): Json<UpdateBio>,
) -> Result<(), AppError> {
    let id = auth_user_id(&user, &client, &token_ids).await?;
    sqlx::query!("UPDATE users SET bio = ? where github_id = ?", bio, id).execute(&db).await?;

    Ok(())
}
