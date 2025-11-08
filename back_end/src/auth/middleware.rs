use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Extension,
};
use axum_extra::extract::cookie::CookieJar;
use tracing::instrument;

use crate::{
    auth::{ACCESS_COOKIE, SharedTokenInfo, TokenHeaders, WithTokenHeaders, get_auth_user},
    error::AppError, github::GithubClient
};

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub github_id: i32,
    pub access_token: String,
    pub refresh_token: String,
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
    client: State<GithubClient>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let WithTokenHeaders(maybe_auth_user, token_headers) = match get_auth_user(token_info, client, &jar).await? {
        Some(user) => user.map(Some),
        None => Default::default(),
    };

    req.extensions_mut().insert(maybe_auth_user);

    append_token_headers(next.run(req).await, token_headers)
}

#[instrument(skip_all)]
pub async fn auth_middleware(
    token_info: Extension<SharedTokenInfo>,
    client: State<GithubClient>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let WithTokenHeaders(auth_user, token_headers) = get_auth_user(token_info, client, &jar)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // TODO: does this actually do anything?
    let _ = jar.remove(ACCESS_COOKIE);

    req.extensions_mut().insert(auth_user);

    append_token_headers(next.run(req).await, token_headers)
}

pub async fn redirect_auth_middleware(
    token_info: Extension<SharedTokenInfo>,
    client: State<GithubClient>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // TODO: probably shouldn't return AppError on a public route (/editor)

    match get_auth_user(token_info, client, &jar).await? {
        Some(WithTokenHeaders(user, headers)) => {
            req.extensions_mut().insert(user);

            append_token_headers(next.run(req).await, headers)
        }
        None => Ok(Redirect::to("/").into_response())
    }
}
