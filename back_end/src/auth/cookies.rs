use std::ops::Deref;

use axum::{
    http::{HeaderName, HeaderValue},
    response::{IntoResponseParts, ResponseParts},
};
use chrono::{DateTime, Utc};
use reqwest::header::SET_COOKIE;

use crate::github::access_tokens::Tokens;

/// Name of the cookie that stores the access token
pub const ACCESS_COOKIE: &str = "access-token";
/// Name of the cookie that stores the refresh token
pub const REFRESH_COOKIE: &str = "refresh-token";

// Headers that set the access and refresh token in cookies
pub struct TokenHeaders {
    access_header: (HeaderName, HeaderValue),
    refresh_header: (HeaderName, HeaderValue),
}

impl IntoResponseParts for TokenHeaders {
    type Error = ();

    fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        res.headers_mut()
            .extend([self.access_header, self.refresh_header]);

        Ok(res)
    }
}

impl From<Tokens> for TokenHeaders {
    fn from(tokens: Tokens) -> Self {
        Self::from(&tokens)
    }
}

impl From<&Tokens> for TokenHeaders {
    fn from(tokens: &Tokens) -> Self {
        Self::new(
            &tokens.access_token,
            &tokens.refresh_token,
            tokens.refresh_expiry,
        )
    }
}

impl TokenHeaders {
    pub fn new(
        access_token: &str,
        refresh_token: &str,
        refresh_expiry_date: DateTime<Utc>,
    ) -> Self {
        Self {
            access_header: (
                SET_COOKIE,
                HeaderValue::from_str(&format!("{ACCESS_COOKIE}={access_token}; Secure; HttpOnly; SameSite=Strict; Path=/")).unwrap(),
            ),
            refresh_header: (
                SET_COOKIE,
                HeaderValue::from_str(&format!("{REFRESH_COOKIE}={refresh_token}; Secure; HttpOnly; SameSite=Strict; Path=/; Expires={}", refresh_expiry_date.to_rfc2822())).unwrap()
            ),
        }
    }
}
