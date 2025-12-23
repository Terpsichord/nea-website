use axum::{Extension, extract::State};
use axum_extra::extract::CookieJar;
use base64::prelude::{BASE64_STANDARD, Engine};
use chrono::Utc;

use crate::{
    error::{AppError, InvalidAuthError},
    github::{
        GithubClient,
        access_tokens::{TokenRequestType, WithTokens},
    },
};
use middleware::AuthUser;

pub use cookies::*;
pub use token_cache::*;

mod cookies;
pub mod crypto;
pub mod middleware;
mod token_cache;

/// Middleware that gets the currently authenticated user if the API endpoint being requested requires authentication.
/// The user's id is added to the request as an `AuthUser` extension.
pub async fn get_auth_user(
    Extension(token_cache): Extension<TokenCache>,
    State(client): State<GithubClient>,
    jar: &CookieJar,
) -> Result<Option<WithTokens<AuthUser>>, AppError> {
    let Some(access_cookie) = jar.get(ACCESS_COOKIE) else {
        return Ok(None);
    };

    // Decode the access token
    let encrypted_access_token = access_cookie.value().to_string();
    let mut access_token = decode_token(&encrypted_access_token)?;

    // Extract and decode the refresh token
    let Some(refresh_cookie) = jar.get(REFRESH_COOKIE) else {
        Err(InvalidAuthError::MissingRefreshToken)?
    };
    let encrypted_refresh_token = refresh_cookie.value().to_string();
    let mut refresh_token = decode_token(&encrypted_refresh_token)?;

    let maybe_info = token_cache.get_token_info(&encrypted_access_token).await;

    let mut new_tokens = None;

    // If cached token has expired
    if let Some(info) = maybe_info
        && let Some(expiry_date) = info.expiry_date
        && Utc::now() >= expiry_date
    {
        // Get new access and refresh tokens using the current refresh token
        let tokens = client
            .get_tokens(TokenRequestType::Refresh {
                refresh_token: refresh_token.clone(),
                grant_type: (),
            })
            .await?;

        // Update the access and refresh tokens
        access_token.clone_from(&tokens.access_unencrypted);
        refresh_token.clone_from(&tokens.refresh_unencrypted);
        new_tokens = Some(tokens);
    }

    // Get the Github ID from TokenCache if it is cached there
    let github_id = if let Some(info) = maybe_info {
        info.github_id
    // Otherwise fetch it from Github and cache it for future use
    } else {
        let WithTokens(user, tokens) = client.get_user(&access_token, Some(&refresh_token)).await?;
        new_tokens = tokens.or(new_tokens);
        token_cache
            .cache_user_token(&user, encrypted_access_token, None)
            .await;

        user.id
    };

    Ok(Some(WithTokens(
        AuthUser {
            github_id,
            access_token,
            refresh_token,
        },
        new_tokens,
    )))
}

fn decode_token(encrypted_token: &str) -> Result<String, InvalidAuthError> {
    let decoded = BASE64_STANDARD.decode(encrypted_token)?;
    let decrypted = crypto::decrypt(&decoded).map_err(InvalidAuthError::Encryption)?;

    Ok(String::from_utf8(decrypted)?)
}
