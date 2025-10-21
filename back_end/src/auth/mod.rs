use axum::{Extension, extract::State};
use axum_extra::extract::CookieJar;
use base64::prelude::{BASE64_STANDARD, Engine};
use chrono::Utc;

use crate::{
    crypto,
    error::{AppError, InvalidAuthError},
    github::{
        access_tokens::TokenRequestType, GithubClient
    },
    middlewares::auth::AuthUser,
};

pub use cookies::*;
pub use token_info::*;

mod cookies;
mod token_info;

/// Middleware that gets the currently authenticated user if the API endpoint being requested requires authentication.
/// The user's id is added to the request as an `AuthUser` extension.
pub async fn get_auth_user(
    Extension(token_info): Extension<SharedTokenInfo>,
    State(client): State<GithubClient>,
    jar: &CookieJar,
) -> Result<Option<(AuthUser, Option<TokenHeaders>)>, AppError> {
    let Some(access_cookie) = jar.get(ACCESS_COOKIE) else {
        return Ok(None);
    };

    // Decode the access token
    let encrypted_access_token = access_cookie.value().to_string();
    let mut access_token = decode_token(&encrypted_access_token)?;

    let maybe_info = token_info.get_token_info(&encrypted_access_token).await;

    let mut new_token_headers = None;

    // If cached token has expired
    if let Some(info) = maybe_info
        && let Some(expiry_date) = info.expiry_date
        && Utc::now() >= expiry_date
    {
        // Extract refresh token from cookies
        let Some(refresh_cookie) = jar.get(REFRESH_COOKIE) else {
            Err(InvalidAuthError::MissingRefreshToken)?
        };
        let encrypted_refresh_token = refresh_cookie.value().to_string();
        let refresh_token = decode_token(&encrypted_refresh_token)?;

        // Get new access and refresh tokens using the current refresh token
        let tokens = client
            .get_tokens(TokenRequestType::Refresh {
                refresh_token,
                grant_type: (),
            })
            .await?;

        // Update the access and refresh tokens
        new_token_headers = Some((&tokens).into());
        access_token = tokens.access_unencrypted;
    }

    // Get the Github ID from SharedTokenInfo if it is cached there
    let github_id = if let Some(info) = maybe_info {
        info.github_id
    // Otherwise fetch it from Github and cache it for future use
    } else {
        let user = client.get_user(&access_token).await?;
        token_info
            .cache_user_token(&user, encrypted_access_token, None)
            .await;

        user.id
    };

    Ok(Some((AuthUser { github_id }, new_token_headers)))
}

fn decode_token(encrypted_token: &str) -> Result<String, InvalidAuthError> {
    let decoded = BASE64_STANDARD.decode(encrypted_token)?;
    let decrypted = crypto::decrypt(&decoded).map_err(InvalidAuthError::Encryption)?;

    Ok(String::from_utf8(decrypted)?)
}
