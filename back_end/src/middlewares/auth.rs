use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Extension,
};
use axum_extra::extract::cookie::CookieJar;
use tracing::{info, instrument};

use crate::{
    auth::{get_auth_user, SharedTokenInfo, TokenHeaders, ACCESS_COOKIE},
    error::AppError
};

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub github_id: i32,
}

// Response is always wrapped in `Ok` as is required by the middleware functions below
#[allow(clippy::unnecessary_wraps)]
fn append_token_headers(resp: Response, headers: Option<TokenHeaders>) -> Result<Response, AppError> {
    Ok(match headers {
        Some(headers) => (headers, resp).into_response(),
        None => resp,
    })
}

pub async fn optional_auth_middleware(
    token_info: Extension<SharedTokenInfo>,
    client: State<reqwest::Client>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let (maybe_auth_user, token_headers) = match get_auth_user(token_info, client, &jar).await? {
        Some((user, headers)) => (Some(user), headers),
        None => (None, None),
    };

    req.extensions_mut().insert(maybe_auth_user);

    append_token_headers(next.run(req).await, token_headers)
}

#[instrument(skip(token_info, client, jar, next))]
pub async fn auth_middleware(
    token_info: Extension<SharedTokenInfo>,
    client: State<reqwest::Client>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    info!("auth middleware");
    let (auth_user, token_headers) = get_auth_user(token_info, client, &jar)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // TODO: does this actually do anything?
    let _ = jar.remove(ACCESS_COOKIE);

    req.extensions_mut().insert(auth_user);

    append_token_headers(next.run(req).await, token_headers)
}

pub async fn redirect_auth_middleware(
    token_info: Extension<SharedTokenInfo>,
    client: State<reqwest::Client>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // TODO: probably shouldn't return AppError on a public route (/editor)

    match get_auth_user(token_info, client, &jar).await? {
        Some((user, headers)) => {
            req.extensions_mut().insert(user);

            append_token_headers(next.run(req).await, headers)
        }
        None => Ok(Redirect::to("/").into_response())
    }
}
