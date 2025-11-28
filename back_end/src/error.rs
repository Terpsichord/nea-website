use anyhow::anyhow;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

pub enum AppError {
    InvalidAuth(InvalidAuthError),
    // TODO: maybe merge this into `AuthFailed`
    GithubAuth(GithubUserError),
    AuthFailed(anyhow::Error),
    Database(sqlx::Error),
    SessionConflict,
    NotFound,
    Unauthorized,
    ProjectExists,
    // try to avoid using this
    // generally prefer creating a new variant instead
    // TODO: remove this probably
    Other(anyhow::Error),
}

#[derive(Deserialize)]
pub struct GithubUserError {
    pub message: String,
    // TODO: probably remove these fields
    // pub documentation_url: String,
    // pub status: String,
}

#[derive(Debug, thiserror::Error)]
pub enum InvalidAuthError {
    #[error("invalid base64")]
    Base64(#[from] base64::DecodeError),
    #[error("invalid aes encryption")]
    Encryption(aes_gcm::Error),
    #[error("invalid utf-8")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("missing refresh token")]
    MissingRefreshToken,
}

impl AppError {
    pub fn auth_failed(err: impl Into<anyhow::Error>) -> Self {
        Self::AuthFailed(err.into())
    }

    pub fn other(err: impl Into<anyhow::Error>) -> Self {
        Self::Other(err.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[allow(clippy::enum_glob_use)]
        use AppError::*;

        let err: anyhow::Error = match self {
            NotFound => return StatusCode::NOT_FOUND.into_response(),
            Unauthorized => return StatusCode::UNAUTHORIZED.into_response(),
            SessionConflict => return StatusCode::CONFLICT.into_response(),
            ProjectExists => return StatusCode::UNPROCESSABLE_ENTITY.into_response(),
            InvalidAuth(e) => e.into(),
            Database(e) => e.into(),
            GithubAuth(e) => anyhow!("Github auth failed: {}", e.message),
            AuthFailed(e) | Other(e) => e,
        };

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            #[cfg(debug_assertions)]
            Json(json!({ "error": err.to_string() })),
            #[cfg(not(debug_assertions))]
            "Something went wrong",
        )
            .into_response()
    }
}

impl From<InvalidAuthError> for AppError {
    fn from(err: InvalidAuthError) -> Self {
        Self::InvalidAuth(err)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound,
            e => Self::Database(e),
        }
    }
}
