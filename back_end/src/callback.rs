use anyhow::Context;
use axum::{extract::{Query, State}, http::{header, HeaderName}, response::Redirect, Extension};
use base64::{prelude::BASE64_STANDARD, Engine};
use serde::{Deserialize, Serialize};

use crate::{api::{AppError, AUTH_COOKIE}, crypto, middlewares::auth::SharedTokenIds, user::{add_user_from_github, fetch_and_cache_github_user}, AppState, Config, CONFIG};

#[derive(Deserialize)]
pub struct UserCode {
    code: String,
}

#[derive(Serialize)]
struct GithubSecrets {
    client_id: &'static str,
    client_secret: &'static str,
}

impl GithubSecrets {
    fn from_config(config: &'static Config) -> Self {
        Self {
            client_id: &config.github_client_id,
            client_secret: &config.github_client_secret,
        }
    }
}

#[derive(Serialize)]
struct AccessTokenRequest {
    #[serde(flatten)]
    secrets: GithubSecrets,
    code: String,
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
    refresh_token_expires_in: u64,
    token_type: String,
}

pub async fn github_callback(
    Query(UserCode { code }): Query<UserCode>,
    Extension(token_ids): Extension<SharedTokenIds>,
    State(state): State<AppState>,
) -> Result<([(HeaderName, String); 1], Redirect), AppError> {

    let params = AccessTokenRequest {
        code,
        secrets: GithubSecrets::from_config(&CONFIG),
    };

    let text = state
        .client
        .post("https://github.com/login/oauth/access_token")
        .form(&params)
        .send()
        .await?
        .text()
        .await?;

    let AccessTokenResponse { access_token, .. } =
        serde_urlencoded::from_str::<AccessTokenResponse>(&text)
            .with_context(|| format!("failed to decode AccessTokenRequest from: {text}"))?;
    let encrypted_token = BASE64_STANDARD.encode(crypto::encrypt(access_token.as_bytes()));

    let user = fetch_and_cache_github_user(&access_token, &state.client, &encrypted_token, &token_ids).await?;
    add_user_from_github(user, &state.db).await?;

    let headers = [(
        header::SET_COOKIE,
        format!("{AUTH_COOKIE}={encrypted_token}; Secure; HttpOnly; SameSite=Strict; Path=/"),
    )];
    Ok((headers, Redirect::to("/")))
}

