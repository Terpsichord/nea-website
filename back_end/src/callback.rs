use axum::{
    extract::{Query, State}, response::Redirect, Extension
};
use serde::Deserialize;
use tracing::{info, instrument};

use crate::{
    auth::{
        get_tokens_with_unencrypted, token_headers, SharedTokenInfo, TokenHeaders,
        TokenRequestType,
    },
    error::AppError,
    user::{add_user_from_github, fetch_and_cache_github_user},
    AppState, 
};

#[derive(Deserialize)]
pub struct UserCode {
    code: String,
}

#[instrument(skip(state))]
pub async fn github_callback(
    Query(UserCode { code }): Query<UserCode>,
    Extension(token_info): Extension<SharedTokenInfo>,
    State(state): State<AppState>,
) -> Result<(TokenHeaders, Redirect), AppError> {
    info!("handling Github auth callback");

    let (
        [(access_token, _access_expiry_date), (refresh_token, refresh_expiry_date)],
        access_token_unencrypted,
    ) = get_tokens_with_unencrypted(&state.client, TokenRequestType::Callback { code }).await?;

    // TODO: cache each refresh token's `expires_in` to avoid making a request with an already expired refresh token

    let user = fetch_and_cache_github_user(
        &access_token_unencrypted,
        &state.client,
        &access_token,
        &token_info,
    )
    .await?;

    add_user_from_github(user, &state.db).await?;

    let headers = token_headers(&access_token, &refresh_token, refresh_expiry_date);
    Ok((headers, Redirect::to("/")))
}
