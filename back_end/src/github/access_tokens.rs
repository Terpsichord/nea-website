use std::ops::Deref;

use super::GithubClient;

use anyhow::Context;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize, Serializer};

use crate::{
    CONFIG, Config,
    auth::crypto::Aes256Gcm,
    error::AppError,
};

// `GithubSecrets` stores the details needed to authenticate the GitHub App when making requests to the GitHub API
#[derive(Serialize)]
struct GithubSecrets {
    client_id: &'static str,
    client_secret: &'static str,
}

impl GithubSecrets {
    // `GithubSecrets` is initialised using environment variables which are loaded into `Config`
    fn from_config(config: &'static Config) -> Self {
        Self {
            client_id: &config.github_client_id,
            client_secret: &config.github_client_secret,
        }
    }
}

/// The type of token request that is being made
#[derive(Serialize)]
#[serde(untagged)]
pub enum TokenRequestType {
    // Token request made after receiving callback from Github when signing-in
    Callback {
        code: String,
    },
    // Token request made when the user's auth token has expired and must be refreshed
    Refresh {
        #[serde(serialize_with = "refresh_grant")]
        grant_type: (),
        refresh_token: String,
    },
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn refresh_grant<S: Serializer>(_: &(), s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str("refresh_token")
}

// Data sent via JSON for GitHub access token request
#[derive(Serialize)]
struct AccessTokenRequest {
    #[serde(flatten)]
    secrets: GithubSecrets,
    #[serde(flatten)]
    req_type: TokenRequestType,
}

// Data received via JSON in access token request
#[derive(Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
    refresh_token_expires_in: u64,
}

// Struct used to pass new tokens up the call stack after tokens have been refreshed
#[derive(Default)]
#[must_use]
pub struct WithTokens<T>(pub T, pub Option<Tokens>);

impl<T> WithTokens<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> WithTokens<U> {
        WithTokens(f(self.0), self.1)
    }
}

impl<T> Deref for WithTokens<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// `Tokens` stores access and refresh tokens (both encrypted and unencrytped) and their expiries for a single user 
pub struct Tokens {
    pub access_token: String,
    pub access_expiry: DateTime<Utc>,
    pub refresh_token: String,
    pub refresh_expiry: DateTime<Utc>,
    pub access_unencrypted: String,
    pub refresh_unencrypted: String,
}

impl Tokens {
    /// Returns the pair of unencrypted access and refresh tokens
    pub fn unencrypted(&self) -> (&str, &str) {
        (&self.access_unencrypted, &self.refresh_unencrypted)
    }
}

impl GithubClient {
    /// Fetches the access token and refresh token from Github, and encrypts them so they can be stored in cookies
    ///
    /// Returns the encrypted access and refresh tokens (with expiries), and the unencrypted access token
    ///
    /// This function is used after a user has signed-in (`TokenRequestType::callback`), or their access token has expired and needs to be refreshed (`TokenRequestType::Refresh`)
    pub async fn get_tokens(&self, req_type: TokenRequestType) -> Result<Tokens, AppError> {
        // Initialise the parameters to the access token request
        let params = AccessTokenRequest {
            secrets: GithubSecrets::from_config(&CONFIG),
            req_type,
        };

        // Send HTTP request for access token API request, and store HTTP response in `text`
        let text = self
            .client
            .post("https://github.com/login/oauth/access_token")
            .form(&params)
            .send()
            .await
            .map_err(AppError::auth_failed)?
            .text()
            .await
            .map_err(AppError::auth_failed)?;

        // decode tokens from http body 
        let AccessTokenResponse {
            access_token,
            expires_in,
            refresh_token,
            refresh_token_expires_in,
        } = serde_urlencoded::from_str::<AccessTokenResponse>(&text)
            .with_context(|| format!("failed to decode AccessTokenRequest from: {text}"))
            .map_err(AppError::auth_failed)?;

        // use AES to produce the encrypted tokens
        let encrypted_access_token = Aes256Gcm::encrypt_base64(access_token.as_bytes());
        let encrypted_refresh_token = Aes256Gcm::encrypt_base64(refresh_token.as_bytes());

        // this is okay to cast as according to the docs, this value should always be 15897600 (6 months)
        // https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens#refreshing-a-user-access-token-with-a-refresh-token

        // calculate expiry dates
        #[allow(clippy::cast_possible_wrap)]
        let access_expiry_date = Utc::now() + Duration::seconds(expires_in as i64);
        #[allow(clippy::cast_possible_wrap)]
        let refresh_expiry_date = Utc::now() + Duration::seconds(refresh_token_expires_in as i64);

        Ok(Tokens {
            access_token: encrypted_access_token,
            access_expiry: access_expiry_date,
            refresh_token: encrypted_refresh_token,
            refresh_expiry: refresh_expiry_date,
            access_unencrypted: access_token,
            refresh_unencrypted: refresh_token,
        })
    }
}
