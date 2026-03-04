use axum::{
    Extension,
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;
use tracing::instrument;

use crate::{
    AppState, GITHUB_APP_SLUG,
    auth::{TokenCache, TokenHeaders},
    error::AppError,
    github::access_tokens::{TokenRequestType, WithTokens},
};

// Code passed by GitHub in the callback URL's query params
#[derive(Deserialize)]
pub struct UserCode {
    code: String,
}

#[instrument(skip(token_cache, client, db))]
/// Callback that the user is redirected to after authenticating with Github
pub async fn github_callback(
    Query(UserCode { code }): Query<UserCode>,
    Extension(token_cache): Extension<TokenCache>,
    State(AppState { client, db, .. }): State<AppState>,
) -> Result<(Option<TokenHeaders>, Redirect), AppError> {
    // get the user's access and refresh tokens using the Callback code returned by the GitHub OAuth redirect
    let tokens = client
        .get_tokens(TokenRequestType::Callback { code })
        .await?;

    // TODO: cache each refresh token's `expires_in` to avoid making a request with an already expired refresh token

    // get information about the user's GitHub account using the access token
    // (we don't pass a refresh token as the access token can't have already expired)
    let WithTokens(user, _) = client.get_user(&tokens.access_unencrypted, None).await?;

    // add the user to the database if needed
    db.add_user(&user).await?;

    // create the HTTP headers that will be added to the server response to set the cookies for auth
    let headers = TokenHeaders::from(&tokens);

    // store information about the user under the access token in the token LRU cache
    token_cache
        .cache_user_token(&user, tokens.access_token, Some(tokens.access_expiry))
        .await;

    Ok(
        // if the user already has the GitHub App installed, redirect to the dashboard
        if client.user_installed(&tokens.access_unencrypted).await? {
            (Some(headers), Redirect::to("/"))
        // otherwise, redirect to the GitHub page where the App can be added to the account
        } else {
            (
                None,
                Redirect::to(&format!(
                    "https://github.com/apps/{GITHUB_APP_SLUG}/installations/new"
                )),
            )
        },
    )
}
