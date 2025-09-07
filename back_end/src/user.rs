use crate::{
    auth::{SharedTokenInfo, TokenInfo, ACCESS_EXPIRY}, error::{AppError, GithubUserError}, 
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Deserialize)]
pub struct GithubUser {
    pub id: i32,
    #[serde(rename = "login")]
    username: String,
    avatar_url: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum GithubUserResponse {
    User(GithubUser),
    Error(GithubUserError),
}

/// Fetches information about the Github user using the access token, and caches the user's id with the encrypted token
/// 
/// Returns the user info on a successful fetch
pub async fn fetch_and_cache_github_user(
    access_token: &str,
    client: &reqwest::Client,
    encrypted_token: &str,
    token_info: &SharedTokenInfo,
) -> Result<GithubUser, AppError> {
    let res = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(AppError::auth_failed)?
        .json::<GithubUserResponse>()
        .await
        .map_err(AppError::auth_failed)?;

    let user = match res {
        GithubUserResponse::User(user) => user,
        GithubUserResponse::Error(error) => return Err(AppError::GithubAuth(error)),
    };

    token_info
        .write()
        .await
        .insert(encrypted_token.to_string(), TokenInfo {
            github_id: user.id,
            expiry_date: Utc::now() + ACCESS_EXPIRY, 
        });

    Ok(user)
}

pub async fn add_user_from_github(user: GithubUser, database: &PgPool) -> sqlx::Result<()> {
    let existing_user = sqlx::query!("SELECT * FROM users WHERE github_id = $1", user.id)
        .fetch_optional(database)
        .await?;

    if existing_user.is_none() {
        sqlx::query!(
            "INSERT INTO users (github_id, username, picture_url) VALUES ($1, $2, $3)",
            user.id,
            user.username,
            user.avatar_url,
        )
        .execute(database)
        .await?;
    }

    Ok(())
}

#[derive(Serialize, FromRow, sqlx::Type)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub username: String,
    pub picture_url: String,
    pub bio: String,
    pub join_date: NaiveDate,
}
