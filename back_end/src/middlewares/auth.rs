use std::{collections::HashMap, sync::{Arc, RwLock}};
use anyhow::{anyhow, Context};
use axum::{
    extract::Request,
    middleware::Next,
    response::Response, Extension,
};
use axum_extra::extract::cookie::CookieJar;
use base64::{prelude::BASE64_STANDARD, Engine};

use crate::{api::{AppError, AUTH_COOKIE}, crypto};

#[derive(Clone)]
pub struct AuthUser {
    pub encrypted_token: String,
    pub token: String,
    pub id: Option<i64>,
}

pub async fn auth_middleware(
    Extension(token_ids): Extension<SharedTokenIds>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let encrypted_token = jar
        .get(AUTH_COOKIE)
        .context(format!("missing {AUTH_COOKIE} cookie"))?
        .value()
        .to_string();
    let token = String::from_utf8(crypto::decrypt(&BASE64_STANDARD.decode(&encrypted_token)?).map_err(|_| anyhow!("failed to decrypt token"))?)?;

    let id = token_ids.read().unwrap().get(&encrypted_token).copied();
    req.extensions_mut().insert(AuthUser { encrypted_token, token, id });

    // TODO: does this actually do anything?
    let _ = jar.remove(AUTH_COOKIE);

    Ok(next.run(req).await)
}

pub type SharedTokenIds = Arc<RwLock<HashMap<String, i64>>>;