use std::error::Error;

use crate::middlewares::auth::{AuthUser, SharedTokenIds};
use anyhow::{anyhow, bail};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

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
    Error {
        message: String,
        documentation_url: String,
        status: String,
    },
}

pub async fn fetch_and_cache_github_user(
    access_token: &str,
    client: &reqwest::Client,
    encrypted_token: &str,
    token_ids: &SharedTokenIds,
) -> anyhow::Result<GithubUser> {
    let res = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(|e| anyhow!("failed to auth github user: {:?}", e.source().unwrap_or(&e)))?
        .json::<GithubUserResponse>()
        .await
        .map_err(|err| anyhow!("failed to decode GithubUserResponse: {}", err))?;

    let user = match res {
        GithubUserResponse::User(user) => user,
        GithubUserResponse::Error { message, .. } => bail!("failed to auth github user: {}", message),
    };

    token_ids
        .write()
        .unwrap()
        .insert(encrypted_token.to_string(), user.id);

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

pub async fn auth_user_id(
    user: &AuthUser,
    client: &reqwest::Client,
    token_ids: &SharedTokenIds,
) -> anyhow::Result<i32> {
    Ok(match user.id {
        Some(id) => id,
        None => {
            fetch_and_cache_github_user(&user.token, client, &user.encrypted_token, token_ids)
                .await?
                .id
        }
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub username: String,
    pub picture_url: String,
    pub bio: String,
    pub join_date: NaiveDate,
}
