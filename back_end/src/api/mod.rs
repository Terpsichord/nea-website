use axum::{
    Json, Router,
    http::{HeaderName, header},
    routing::{get, post},
};
use chrono::{NaiveDate, DateTime, Utc};
use serde::Serialize;
use sqlx::prelude::FromRow;

use crate::{AppState, db::Project};

mod comment;
mod follow;
mod profile;
mod project;
mod search;
mod user;

pub fn api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(profile::profile_router(state.clone()))
        .merge(user::user_router())
        .merge(follow::follow_router(state.clone()))
        .merge(project::project_router(state.clone()))
        .merge(comment::comment_router(state))
}

#[derive(Debug, Serialize, FromRow, sqlx::Type)]
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
