use axum::{
    extract::{Path, State}, http::{header, HeaderName, StatusCode}, middleware, response::{IntoResponse, Response}, routing::{get, post}, Extension, Json, Router
};
use axum_extra::extract::CookieJar;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::{middlewares::auth::{auth_middleware, AuthUser}, AppState};

mod follow;
mod profile;
mod user;

pub const AUTH_COOKIE: &str = "access-token";

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("{}", &self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": self.0.to_string()})),
        )
            .into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

pub fn api_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(profile::profile_route(state.clone()))
        .merge(user::user_route())
        .merge(follow::follow_route(state))
        .route("/auth", get(auth_handler))
        .route("/signout", post(sign_out))
}

async fn auth_handler(jar: CookieJar) -> Json<Value> {
    Json(json!({ "isAuth": jar.get(AUTH_COOKIE).is_some() }))
}

async fn sign_out() -> [(HeaderName, String); 1] {
    [(
        header::SET_COOKIE,
        format!("{AUTH_COOKIE}=; Max-Age=0; Path=/"),
    )]
}
