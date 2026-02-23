use axum::{
    Json, Router, http::{HeaderName, header, StatusCode}, routing::{get, post} 
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::prelude::FromRow;

use crate::auth::ACCESS_COOKIE;
use crate::{AppState, db::Project};

mod comment;
mod follow;
mod profile;
mod project;
mod recs;
mod search;
mod user;

// This module contains all of code to create the API routes as described in the Design section.

pub fn api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(profile::profile_router(state.clone()))
        .merge(user::user_router())
        .merge(follow::follow_router(state.clone()))
        .merge(project::project_router(state.clone()))
        .merge(comment::comment_router(state.clone()))
        .merge(recs::rec_router(state))
        .fallback(api_not_found)
}

async fn api_not_found() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "API route not found")
}

#[derive(Debug, Deserialize, Serialize, FromRow, sqlx::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub title: String,
    pub username: String,
    pub picture_url: String,
    pub repo_name: String,
    pub readme: String,
    pub tags: Vec<String>,
    pub like_count: i64,
}

#[derive(Debug, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub info: ProjectInfo,
    pub github_url: String,
    pub upload_time: DateTime<Utc>,
    pub public: bool,
    pub owned: bool,
}

impl From<Project> for ProjectResponse {
    fn from(project: Project) -> Self {
        Self {
            info: project.info,
            github_url: project.github_url,
            upload_time: project.upload_time,
            public: project.public,
            owned: project.owned,
        }
    }
}

#[derive(Serialize, FromRow, sqlx::Type)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub username: String,
    pub picture_url: String,
    pub bio: String,
    pub join_date: NaiveDate,
}
