use axum::{
    Extension,
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use tracing::instrument;

use crate::{
    auth::{ACCESS_COOKIE, TokenCache, TokenHeaders, get_auth_user},
    error::AppError,
    github::{
        GithubClient,
        access_tokens::{Tokens, WithTokens},
    },
};

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub github_id: i32,
    pub access_token: String,
    pub refresh_token: String,
}

// Response is always wrapped in `Ok` as is required by the middleware functions below
#[allow(clippy::unnecessary_wraps)]
fn append_token_headers(resp: Response, tokens: Option<Tokens>) -> Result<Response, AppError> {
    Ok(match tokens {
        Some(tokens) => (TokenHeaders::from(tokens), resp).into_response(),
        None => resp,
    })
}

pub async fn optional_auth_middleware(
    token_cache: Extension<TokenCache>,
    client: State<GithubClient>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let WithTokens(maybe_auth_user, tokens) = get_auth_user(token_cache, client, &jar)
        .await?
        .map_or_else(Default::default, |user| user.map(Some));

    req.extensions_mut().insert(maybe_auth_user);

    append_token_headers(next.run(req).await, tokens)
}

#[instrument(skip_all)]
pub async fn auth_middleware(
    token_cache: Extension<TokenCache>,
    client: State<GithubClient>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let WithTokens(auth_user, tokens) = get_auth_user(token_cache, client, &jar)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // TODO: does this actually do anything?
    let _ = jar.remove(ACCESS_COOKIE);

    req.extensions_mut().insert(auth_user);

    append_token_headers(next.run(req).await, tokens)
}

pub async fn redirect_auth_middleware(
    token_cache: Extension<TokenCache>,
    client: State<GithubClient>,
    jar: CookieJar,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // TODO: probably shouldn't return AppError on a public route (/editor)

    match get_auth_user(token_cache, client, &jar).await? {
        Some(WithTokens(user, tokens)) => {
            req.extensions_mut().insert(user);

            append_token_headers(next.run(req).await, tokens)
        }
        None => Ok(Redirect::to("/").into_response()),
    }
}
