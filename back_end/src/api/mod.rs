use std::{fmt::Display, fs, io};

use axum::{
    Json, Router,
    http::{HeaderName, header},
    routing::{get, post},
};
use axum_extra::extract::CookieJar;
use chrono::{NaiveDate, DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::prelude::FromRow;

use crate::auth::ACCESS_COOKIE;
use crate::{AppState, db::Project};

mod comment;
mod follow;
mod profile;
mod project;
mod user;

pub fn api_router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(profile::profile_router(state.clone()))
        .merge(user::user_router())
        .merge(follow::follow_router(state.clone()))
        .merge(project::project_router(state.clone()))
        .merge(comment::comment_router(state))
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

#[derive(Serialize, FromRow, sqlx::Type)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectResponse {
    #[serde(flatten)]
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ProjectLang {
    #[serde(rename = "py")]
    Python,
    #[serde(rename = "js")]
    JavaScript,
    #[serde(rename = "ts")]
    TypeScript,
    #[serde(rename = "rs")]
    Rust,
    #[serde(rename = "c")]
    C,
    #[serde(rename = "cpp")]
    CPlusPlus,
    #[serde(rename = "cs")]
    CSharp,
    #[serde(rename = "sh")]
    Bash,
    #[serde(rename = "java")]
    Java,
}

impl Display for ProjectLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ProjectLang::Python => "py",
            ProjectLang::JavaScript => "js",
            ProjectLang::TypeScript => "ts",
            ProjectLang::Rust => "rs",
            ProjectLang::C => "c",
            ProjectLang::CPlusPlus => "cpp",
            ProjectLang::CSharp => "cs",
            ProjectLang::Bash => "sh",
            ProjectLang::Java => "java",
        })
    }
}

impl ProjectLang {
    const LANG_PATH: &'static str = "./back_end/languages";

    pub fn get_project_toml(self) -> io::Result<String> {
        fs::read_to_string(format!("{}/{}/project.toml", Self::LANG_PATH, self))
    }

    pub fn get_initial_file(self) -> io::Result<(&'static str, String)> {
        let name = match self {
            ProjectLang::Python => "main.py",
            ProjectLang::JavaScript => "main.js",
            ProjectLang::TypeScript => "main.ts",
            ProjectLang::C => "main.c",
            ProjectLang::CPlusPlus => "main.cpp",
            ProjectLang::Bash => "main.sh",
            ProjectLang::Java => "main.java",
            // these languages have readmes with instructions on how to get started 
            ProjectLang::Rust | ProjectLang::CSharp => "README.md",
        };

        let content = fs::read_to_string(format!("{}/{}/init", Self::LANG_PATH, self))?;

    
        Ok((name, content))
    }
}