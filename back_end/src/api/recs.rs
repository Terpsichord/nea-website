use std::fs;

use anyhow::Error;
use axum::{Extension, Router, extract::State, middleware, routing::get};
use chrono::{Duration, Utc};
use serde::Serialize;
use sqlx::types::Json;
use tokio::process::Command;
use tracing::error;

use crate::{
    AppState,
    api::ProjectInfo,
    auth::middleware::{AuthUser, optional_auth_middleware},
    db::DatabaseConnector,
    error::AppError,
};

pub fn rec_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/rec", get(get_recommendations))
        .layer(middleware::from_fn_with_state(state, optional_auth_middleware))
}

// After 6 hours, a user's recommendations are refreshed
// to account for a changes in their interaction history
const REC_REFRESH_PERIOD: Duration = Duration::hours(6);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Category {
    name: String,
    projects: Json<Vec<ProjectInfo>>,
}

async fn get_recommendations(
    Extension(auth_user): Extension<Option<AuthUser>>,
    State(db): State<DatabaseConnector>,
) -> Result<axum::Json<Vec<Category>>, AppError> {
    let recs = if let Some(AuthUser { github_id, .. }) = auth_user {
        let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE github_id = $1", github_id)
            .fetch_one(&*db)
            .await?;

        let last_update = sqlx::query_scalar!(
            r#"
            SELECT created_at
            FROM recs
            WHERE user_id = $1
            LIMIT 1
            "#,
            user_id
        )
        .fetch_optional(&*db)
        .await?;

        match last_update {
            // only update recs if refresh period has been exceeded
            Some(timestamp) if Utc::now() - timestamp < REC_REFRESH_PERIOD => {},
            _ => update_recs(user_id).await.map_err(AppError::other)?,
        }

        sqlx::query_as!(
            Category,
            r#"
            SELECT
                c.name as "name!: String",
                COALESCE(
                    jsonb_agg(
                        jsonb_build_object(
                            'title', p.title,
                            'username', pi.username,
                            'pictureUrl', pi.picture_url,
                            'repoName', p.repo_name,
                            'readme', p.readme,
                            'tags', pi.tags,
                            'likeCount', pi.like_count
                        )
                    ) FILTER (WHERE p.id IS NOT NULL),
                    '[]'
                ) AS "projects!: Json<Vec<ProjectInfo>>"
            FROM rec_categories C
            LEFT JOIN recs r ON r.category_id = c.id
            LEFT JOIN projects p ON p.id = r.project_id
            LEFT JOIN project_info pi ON pi.id = p.id
            GROUP BY c.id, c.name
            "#
        )
        .fetch_all(&*db)
        .await?
    } else {
        // TODO: return recs for users not signed in (trending projects or smth)
        vec![]
    };

    Ok(axum::Json(recs))
}

const RECSYS_PATH: &str = "./recsys/";

async fn update_recs(user_id: i32) -> anyhow::Result<()> {
    // run the recsys inference python script
    let output = Command::new("venv/bin/python3")
        .args(["inference.py", &user_id.to_string()])
        .current_dir(fs::canonicalize(RECSYS_PATH)?)
        .output()
        .await
        .map_err(AppError::other)?;

    // check for and return any errors after running the script
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        error!("{}", &err_msg);
        return Err(Error::msg(err_msg));
    }

    Ok(())
}
