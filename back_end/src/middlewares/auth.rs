use anyhow::{anyhow, Context};
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
    api::{AppError, AUTH_COOKIE},
    crypto,
    user::fetch_and_cache_github_user,
    AppState,
};

#[derive(Clone)]
pub struct AuthUser {
    pub github_id: i32,
}

pub async fn auth_middleware(
    Extension(token_ids): Extension<SharedTokenIds>,
    State(client): State<reqwest::Client>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let encrypted_token = jar
        .get(AUTH_COOKIE)
        .context(format!("missing {AUTH_COOKIE} cookie"))?
        .value()
        .to_string();

    let token = String::from_utf8(
        crypto::decrypt(&BASE64_STANDARD.decode(&encrypted_token)?)
            .map_err(|_| anyhow!("failed to decrypt token"))?,
    )?;

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

    req.extensions_mut().insert(AuthUser { github_id });

    // TODO: does this actually do anything?
    let _ = jar.remove(AUTH_COOKIE);

    Ok(next.run(req).await)
}

pub type SharedTokenIds = Arc<RwLock<HashMap<String, i32>>>;
