use axum::{
    extract::{Path, State}, routing::get, Json, Router
};
use chrono::NaiveDateTime;
use serde::Serialize;
use sqlx::{FromRow, PgPool};

use crate::{user::UserResponse, AppState};

use super::{user::ProjectInfo, AppError};

pub fn project_route() -> Router<AppState> {
    Router::new().route("/project/{username}/{repo_name}", get(get_project))
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct ProjectResponse {
    #[serde(flatten)]
    info: ProjectInfo,
    github_url: String,
    upload_time: NaiveDateTime,
}

async fn get_project(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<PgPool>,
) -> Result<Json<ProjectResponse>, AppError> {
    let project: ProjectResponse = sqlx::query_as!(ProjectResponse, r#"
        SELECT ROW(p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags) as "info!: ProjectInfo", pi.github_url as "github_url!", p.upload_time
        FROM projects p
        INNER JOIN project_info pi ON pi.id = p.id
        WHERE pi.username = $1
        AND p.repo_name = $2
    "#, username, repo_name).fetch_one(&db).await?;

    Ok(Json(project))
}
