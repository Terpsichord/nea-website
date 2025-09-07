use axum::{
    http::{header, HeaderName},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use serde_json::{json, Value};

use crate::AppState;
use crate::auth::ACCESS_COOKIE;

mod follow;
mod profile;
mod project;
mod user;

pub fn api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(profile::profile_router(state.clone()))
        .merge(user::user_router())
        .merge(follow::follow_router(state.clone()))
        .merge(project::project_router(state))
        .route("/auth", get(auth_handler))
        .route("/signout", post(sign_out))
}

async fn auth_handler(jar: CookieJar) -> Json<Value> {
    Json(json!({ "isAuth": jar.get(ACCESS_COOKIE).is_some() }))
}

async fn sign_out() -> [(HeaderName, String); 1] {
    [(
        header::SET_COOKIE,
        format!("{ACCESS_COOKIE}=; Max-Age=0; Path=/"),
    )]
}
