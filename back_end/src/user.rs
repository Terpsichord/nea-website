use std::error::Error;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use crate::middlewares::auth::{AuthUser, SharedTokenIds};

#[derive(Serialize, Deserialize)]
pub struct GithubUser {
    pub id: i64,
    #[serde(rename = "login")]
    username: String,
    avatar_url: String,
}

pub async fn fetch_and_cache_github_user(access_token: &str, client: &reqwest::Client, encrypted_token: &str, token_ids: &SharedTokenIds) -> anyhow::Result<GithubUser> {
    let user = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| anyhow!("failed to auth github user: {:?}", e.source().unwrap_or(&e)))?
        .json::<GithubUser>()
        .await
        .map_err(|err| anyhow!("failed to decode User: {}", err))?;

    token_ids.write().unwrap().insert(encrypted_token.to_string(), user.id);

    Ok(user)
}

pub async fn add_user_from_github(
    user: GithubUser,
    database: &MySqlPool,
) -> sqlx::Result<()> {
    let existing_user = sqlx::query!("SELECT * FROM users WHERE github_id = ?", user.id)
        .fetch_optional(database)
        .await?;

    if existing_user.is_none() {
        sqlx::query!(
            "INSERT INTO users (github_id, username, picture_url) VALUES (?, ?, ?)",
            user.id,
            user.username,
            user.avatar_url,
        )
        .execute(database)
        .await?;
    }

    Ok(())
}

pub async fn auth_user_id(user: &AuthUser, client: &reqwest::Client, token_ids: &SharedTokenIds) -> anyhow::Result<i64> {
    Ok(match user.id {
        Some(id) => id,
        None => fetch_and_cache_github_user(&user.token, client, &user.encrypted_token, token_ids).await?.id
    })
}