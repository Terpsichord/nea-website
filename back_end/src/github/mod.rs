use std::iter;

use axum::http::HeaderValue;
use reqwest::{RequestBuilder, Response, StatusCode, header::USER_AGENT};
use serde::Deserialize;

use crate::{
    auth::TokenHeaders,
    error::{AppError, GithubUserError},
    github::access_tokens::TokenRequestType,
};

pub mod access_tokens;

#[derive(Clone, Debug)]
pub struct GithubClient {
    client: reqwest::Client,
}

impl Default for GithubClient {
    fn default() -> Self {
        Self {
            client: reqwest::Client::builder()
                .default_headers(
                    iter::once((USER_AGENT, HeaderValue::from_static(Self::USER_AGENT))).collect(),
                )
                .build()
                .unwrap(),
        }
    }
}

#[derive(Deserialize)]
pub struct GithubUser {
    pub id: i32,
    #[serde(rename = "login")]
    pub username: String,
    pub avatar_url: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum GithubUserResponse {
    User(GithubUser),
    Error(GithubUserError),
}

impl GithubClient {
    const USER_AGENT: &str = "nea-website";

    async fn send_authenticated(
        &self,
        mut req: RequestBuilder,
        access_token: &str,
        refresh_token: Option<&str>,
    ) -> Result<(Response, Option<TokenHeaders>), AppError> {
        let req_clone = req.try_clone();

        req = req.header("Authorization", format!("Bearer {access_token}"));

        let mut resp = req.send().await.map_err(AppError::other)?;

        let mut token_headers = None;
        if let Some(refresh) = refresh_token
            && resp.status() == StatusCode::UNAUTHORIZED
        {
            let new_tokens = self
                .get_tokens(TokenRequestType::Refresh {
                    refresh_token: refresh.to_string(),
                    grant_type: (),
                })
                .await?;
            if let Some(req) = req_clone {
                resp = req
                    .header(
                        "Authorization",
                        format!("Bearer {}", new_tokens.access_unencrypted),
                    )
                    .send()
                    .await
                    .map_err(AppError::other)?;
            }
            token_headers = Some(new_tokens.into());
        }

        Ok((resp, token_headers))
    }

    /// Fetches information about the Github user using the access token, and caches the user's id with the encrypted token
    ///
    /// Returns the user info on a successful fetch
    pub async fn get_user(&self, access_token: &str) -> Result<GithubUser, AppError> {
        let (resp, _) = self
            .send_authenticated(
                self.client.get("https://api.github.com/user"),
                access_token,
                None,
            ) // todo: change this from None if this function is called from anywhere outsied of the github callback (and don't ignore the token headers in the line above)
            .await?;

        let user_res = resp
            .json::<GithubUserResponse>()
            .await
            .map_err(AppError::auth_failed)?;

        match user_res {
            GithubUserResponse::User(user) => Ok(user),
            GithubUserResponse::Error(error) => Err(AppError::GithubAuth(error)),
        }
    }
}
