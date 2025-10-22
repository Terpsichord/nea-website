use axum::{
    extract::{Query, State}, response::Redirect, Extension
};
use serde::Deserialize;
use tracing::{info, instrument};

use crate::{
    auth::{
        SharedTokenInfo, TokenHeaders,
    }, error::AppError, github::access_tokens::TokenRequestType, AppState 
};

#[derive(Deserialize)]
pub struct UserCode {
    code: String,
}

#[instrument(skip(client, db))]
/// Callback that the user is redirected to after authenticating with Github
pub async fn github_callback(
    Query(UserCode { code }): Query<UserCode>,
    Extension(token_info): Extension<SharedTokenInfo>,
    State(AppState { client, db }): State<AppState>,
) -> Result<(TokenHeaders, Redirect), AppError> {
    info!("handling Github auth callback");

    let tokens = client.get_tokens(TokenRequestType::Callback { code }).await?;

    // TODO: cache each refresh token's `expires_in` to avoid making a request with an already expired refresh token

    let user = client.get_user(&tokens.access_unencrypted).await?;

    db.add_user(&user).await?;

    let headers = TokenHeaders::from(&tokens);

    token_info.cache_user_token(&user, tokens.access_token, Some(tokens.access_expiry)).await;

    Ok((headers, Redirect::to("/")))
}
