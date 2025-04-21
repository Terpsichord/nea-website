use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    Extension,
};
use axum_extra::extract::cookie::CookieJar;
use base64::{prelude::BASE64_STANDARD, Engine};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    api::AUTH_COOKIE,
    crypto,
    error::{AppError, InvalidAuthError},
    user::fetch_and_cache_github_user,
};

#[derive(Clone)]
pub struct AuthUser {
    pub github_id: i32,
}

// TODO: help?? what was i going to use this for, i forgor 💀
pub async fn optional_auth_middleware(
    token_ids: Extension<SharedTokenIds>,
    client: State<reqwest::Client>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let maybe_auth_user = get_auth_user(token_ids, client, &jar).await?;

    req.extensions_mut().insert(maybe_auth_user);

    Ok(next.run(req).await)
}

pub async fn auth_middleware(
    token_ids: Extension<SharedTokenIds>,
    client: State<reqwest::Client>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_user = get_auth_user(token_ids, client, &jar)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // TODO: does this actually do anything?
    let _ = jar.remove(AUTH_COOKIE);

    req.extensions_mut().insert(auth_user);

    Ok(next.run(req).await)
}

pub async fn get_auth_user(
    Extension(token_ids): Extension<SharedTokenIds>,
    State(client): State<reqwest::Client>,
    jar: &CookieJar,
) -> Result<Option<AuthUser>, AppError> {
    let Some(cookie) = jar.get(AUTH_COOKIE) else {
        return Ok(None);
    };

    let encrypted_token = cookie.value().to_string();
    let token = decode_token(&encrypted_token)?;

    let maybe_id = {
        // needed to satisfy the compiler
        let read_guard = token_ids.read().unwrap();
        read_guard.get(&encrypted_token).copied()
    };

    let github_id = match maybe_id {
        Some(id) => id,
        None => {
            fetch_and_cache_github_user(&token, &client, &encrypted_token, &token_ids)
                .await?
                .id
        }
    };

    Ok(Some(AuthUser { github_id }))
}

fn decode_token(encrypted_token: &str) -> Result<String, InvalidAuthError> {
    let decoded = BASE64_STANDARD.decode(encrypted_token)?;
    let decrypted = crypto::decrypt(&decoded).map_err(InvalidAuthError::Encryption)?;

    Ok(String::from_utf8(decrypted)?)
}

pub type SharedTokenIds = Arc<RwLock<HashMap<String, i32>>>;
