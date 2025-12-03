use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use tokio::sync::RwLock;

use crate::github::GithubUser;

/// Period of time after which the access token expires
pub const ACCESS_EXPIRY: Duration = Duration::hours(8);

#[derive(Copy, Clone, Debug)]
/// A struct containing information associated with a given access token
pub struct TokenInfo {
    /// Github ID of the user that the access token belongs to
    pub github_id: i32,
    /// Date by which the token expires (and must be refreshed)
    pub expiry_date: Option<DateTime<Utc>>,
}

// TODO: Add a Token type that the TokenCache uses instead of String as a key, but also let it dereference directly to a string

#[derive(Clone, Debug, Default)]
/// Table of `TokenInfo` cached with encrypted access tokens
pub struct SharedTokenInfo(Arc<RwLock<HashMap<String, TokenInfo>>>);

impl SharedTokenInfo {
    pub async fn cache_user_token(
        &self,
        user: &GithubUser,
        encrypted_token: String,
        expiry_date: Option<DateTime<Utc>>,
    ) {
        self.0.write().await.insert(
            encrypted_token,
            TokenInfo {
                github_id: user.id,
                expiry_date,
            },
        );
    }

    /// Gets the stored token info for the given token
    ///
    /// Returns None if the token can't be found
    pub async fn get_token_info(&self, token: &str) -> Option<TokenInfo> {
        self.0.read().await.get(token).copied()
    }
}
