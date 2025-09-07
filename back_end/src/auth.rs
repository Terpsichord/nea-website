use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use axum::{
    extract::State, http::{header::SET_COOKIE, HeaderName, HeaderValue}, response::{IntoResponseParts, ResponseParts}, Extension
};
use axum_extra::extract::CookieJar;
use base64::prelude::{Engine, BASE64_STANDARD};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize, Serializer};
use tokio::sync::RwLock;

use crate::{
    crypto,
    error::{AppError, InvalidAuthError},
    middlewares::auth::AuthUser,
    user::fetch_and_cache_github_user,
    Config, CONFIG,
};

/// Name of the cookie that stores the access token
pub const ACCESS_COOKIE: &str = "access-token";
/// Name of the cookie that stores the refresh token
pub const REFRESH_COOKIE: &str = "refresh-token";
/// Period of time after which the access token expires
pub const ACCESS_EXPIRY: Duration = Duration::hours(8);

#[derive(Clone, Debug)]
pub struct TokenInfo {
    pub github_id: i32,
    // TODO: maybe store this as unix timestamp for more efficient storage and comparison (allows TokenInfo to derive Copy probably)
    pub expiry_date: DateTime<Utc>,
}

pub type SharedTokenInfo = Arc<RwLock<HashMap<String, TokenInfo>>>;

// Middleware that gets the currently authenticated user if the API endpoint being requested requires authentication.
// The user's id is added to the request as an `AuthUser` extension.
pub async fn get_auth_user(
    Extension(token_info): Extension<SharedTokenInfo>,
    State(client): State<reqwest::Client>,
    jar: &CookieJar,
) -> Result<Option<(AuthUser, Option<TokenHeaders>)>, AppError> {
    let Some(access_cookie) = jar.get(ACCESS_COOKIE) else {
        return Ok(None);
    };

    let encrypted_access_token = access_cookie.value().to_string();
    let mut access_token = decode_token(&encrypted_access_token)?;

    let token_info_guard = token_info.read().await;
    let maybe_info = token_info_guard.get(&encrypted_access_token);

    let mut new_token_headers = None;
    if let Some(info) = maybe_info {
        if Utc::now() >= info.expiry_date {
            let Some(refresh_cookie) = jar.get(REFRESH_COOKIE) else {
                Err(InvalidAuthError::MissingRefreshToken)?
            };

            let encrypted_refresh_token = refresh_cookie.value().to_string();
            let refresh_token = decode_token(&encrypted_refresh_token)?;

            let [(new_access_token, _access_expiry_date), (new_refresh_token, new_refresh_expiry_date)] =
                get_tokens(
                    &client,
                    TokenRequestType::Refresh {
                        grant_type: (),
                        refresh_token,
                    },
                )
                .await?;

            new_token_headers = Some(token_headers(
                &new_access_token,
                &new_refresh_token,
                new_refresh_expiry_date,
            ));
            access_token = new_access_token;
        }
    }

    let github_id = match maybe_info.map(|info| info.github_id) {
        Some(id) => id,
        None => {
            fetch_and_cache_github_user(
                &access_token,
                &client,
                &encrypted_access_token,
                &token_info,
            )
            .await?
            .id
        }
    };

    Ok(Some((AuthUser { github_id }, new_token_headers)))
}

fn decode_token(encrypted_token: &str) -> Result<String, InvalidAuthError> {
    let decoded = BASE64_STANDARD.decode(encrypted_token)?;
    let decrypted = crypto::decrypt(&decoded).map_err(InvalidAuthError::Encryption)?;

    Ok(String::from_utf8(decrypted)?)
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

/// The type of token request that is being made
#[derive(Serialize)]
#[serde(untagged)]
pub enum TokenRequestType {
    // Token request made after receiving callback from Github when signing-in
    Callback {
        code: String,
    },
    // Token request made when the user's auth token has expired and must be refreshed
    Refresh {
        #[serde(serialize_with = "refresh_grant")]
        grant_type: (),
        refresh_token: String,
    },
}

fn refresh_grant<S: Serializer>(_: &(), s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str("refresh_token")
}

#[derive(Serialize)]
struct AccessTokenRequest {
    #[serde(flatten)]
    secrets: GithubSecrets,
    #[serde(flatten)]
    req_type: TokenRequestType,
}

#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
    refresh_token_expires_in: u64,
}

pub async fn get_tokens(
    client: &reqwest::Client,
    req_type: TokenRequestType,
) -> Result<[(String, DateTime<Utc>); 2], AppError> {
    Ok(get_tokens_with_unencrypted(client, req_type).await?.0)
}

/// Fetches the access token and refresh token from Github, and encrypts them so they can be stored in cookies
pub async fn get_tokens_with_unencrypted(
    client: &reqwest::Client,
    req_type: TokenRequestType,
) -> Result<([(String, DateTime<Utc>); 2], String), AppError> {
    let params = AccessTokenRequest {
        secrets: GithubSecrets::from_config(&CONFIG),
        req_type,
    };

    let text = client
        .post("https://github.com/login/oauth/access_token")
        .form(&params)
        .send()
        .await
        .map_err(AppError::auth_failed)?
        .text()
        .await
        .map_err(AppError::auth_failed)?;

    let AccessTokenResponse {
        access_token,
        expires_in,
        refresh_token,
        refresh_token_expires_in,
    } = serde_urlencoded::from_str::<AccessTokenResponse>(&text)
        .with_context(|| format!("failed to decode AccessTokenRequest from: {text}"))
        .map_err(AppError::auth_failed)?;

    let encrypted_access_token = crypto::encrypt_base64(access_token.as_bytes());
    let encrypted_refresh_token = crypto::encrypt_base64(refresh_token.as_bytes());

    // this is okay to cast as according to the docs, this value should always be 15897600 (6 months)
    // https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens#refreshing-a-user-access-token-with-a-refresh-token

    #[allow(clippy::cast_possible_wrap)]
    let access_expiry_date = Utc::now() + Duration::seconds(expires_in as i64);
    #[allow(clippy::cast_possible_wrap)]
    let refresh_expiry_date = Utc::now() + Duration::seconds(refresh_token_expires_in as i64);

    Ok((
        [
            (encrypted_access_token, access_expiry_date),
            (encrypted_refresh_token, refresh_expiry_date),
        ],
        access_token,
    ))
}

pub struct TokenHeaders {
    access_header: (HeaderName, HeaderValue),
    refresh_header: (HeaderName, HeaderValue),
}

impl IntoResponseParts for TokenHeaders {
    type Error = ();

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        res.headers_mut().extend([self.access_header, self.refresh_header]);
        
        Ok(res)
    }
}

pub fn token_headers(
    access_token: &str,
    refresh_token: &str,
    refresh_expiry_date: DateTime<Utc>,
) -> TokenHeaders {
    TokenHeaders {
        access_header: (
            SET_COOKIE,
            HeaderValue::from_str(&format!("{ACCESS_COOKIE}={access_token}; Secure; HttpOnly; SameSite=Strict; Path=/")).unwrap(),
        ),
        refresh_header: (
            SET_COOKIE,
            HeaderValue::from_str(&format!("{REFRESH_COOKIE}={refresh_token}; Secure; HttpOnly; SameSite=Strict; Path=/; Expires={}", refresh_expiry_date.to_rfc2822())).unwrap()
        ), 
    }
}

