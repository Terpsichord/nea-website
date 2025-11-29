use axum::{
    Extension,
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;
use tracing::{info, instrument};

use crate::{
    AppState,
    auth::{SharedTokenInfo, TokenHeaders},
    error::AppError,
    github::access_tokens::{TokenRequestType, WithTokens},
    GITHUB_APP_SLUG,
};

#[derive(Deserialize)]
pub struct UserCode {
    code: String,
}

#[instrument(skip(token_info, client, db))]
/// Callback that the user is redirected to after authenticating with Github
pub async fn github_callback(
    Query(UserCode { code }): Query<UserCode>,
    Extension(token_info): Extension<SharedTokenInfo>,
    State(AppState { client, db, .. }): State<AppState>,
) -> Result<(Option<TokenHeaders>, Redirect), AppError> {
    info!("handling Github auth callback");

    let tokens = client
        .get_tokens(TokenRequestType::Callback { code })
        .await?;

    // TODO: cache each refresh token's `expires_in` to avoid making a request with an already expired refresh token

    let WithTokens(user, _) = client.get_user(&tokens.access_unencrypted, None).await?;

    db.add_user(&user).await?;

    let headers = TokenHeaders::from(&tokens);

    token_info
        .cache_user_token(&user, tokens.access_token, Some(tokens.access_expiry))
        .await;

    Ok(if client.user_installed(&tokens.access_unencrypted).await? {
        (Some(headers), Redirect::to("/"))
    } else {
        (None, Redirect::to(&format!("https://github.com/apps/{GITHUB_APP_SLUG}/installations/new")))
    })
}
