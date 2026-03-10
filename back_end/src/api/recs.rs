use std::fs;

use anyhow::Error;
use axum::{Extension, Router, extract::State, middleware, routing::{get, post}};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use tokio::process::Command;
use tracing::error;

use crate::{
    AppState,
    api::ProjectInfo,
    auth::middleware::{AuthUser, auth_middleware, optional_auth_middleware},
    db::DatabaseConnector,
    error::AppError,
};

pub fn rec_router(state: AppState) -> Router<AppState> {
    let rec = Router::new()
        .route("/rec", get(get_recommendations))
        .layer(middleware::from_fn_with_state(state.clone(), optional_auth_middleware));

    let interaction = Router::new()
        .route("/interaction", post(add_interaction))
        .layer(middleware::from_fn_with_state(state, auth_middleware));

    Router::new().merge(rec).merge(interaction)

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
        let user_id = db.get_user_id(github_id).await?;

        let last_update = sqlx::query_scalar!(
            r#"
            SELECT created_at
            FROM recs
            WHERE user_id = $1
            ORDER BY created_at DESC
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
            FROM rec_categories c
            LEFT JOIN recs r ON r.category_id = c.id
            LEFT JOIN projects p ON p.id = r.project_id
            LEFT JOIN project_info pi ON pi.id = p.id
            GROUP BY c.id, c.name
            "#
        )
        .fetch_all(&*db)
        .await?
    } else {
        const NUM_TRENDING_PROJECTS: i64 = 10;

        sqlx::query_as!(
            Category,
            r#"
            WITH trending AS (
                SELECT p.id, p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags, pi.like_count
                FROM projects p
                INNER JOIN project_info pi ON pi.id = p.id
                LEFT JOIN (
                    SELECT project_id, COUNT(*) as count
                    FROM interactions
                    GROUP BY project_id
                ) i ON i.project_id = p.id
                WHERE p.public
                ORDER BY COALESCE(i.count, 0) DESC
                LIMIT $1
            )
            SELECT
                'Trending Projects' AS "name!: String",
                jsonb_agg(
                    jsonb_build_object(
                        'title', t.title,
                        'username', t.username,
                        'pictureUrl', t.picture_url,
                        'repoName', t.repo_name,
                        'readme', t.readme,
                        'tags', t.tags,
                        'likeCount', t.like_count
                    )
                ) AS "projects!: Json<Vec<ProjectInfo>>"
            FROM trending t
            "#,
            NUM_TRENDING_PROJECTS,
        )
        .fetch_all(&*db)
        .await?
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

#[derive(Deserialize)]
struct Interaction {
    project_user: String,
    project_repo: String,
    #[serde(rename = "type")]
    type_: String,
}

async fn add_interaction(
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
    State(db): State<DatabaseConnector>,
    axum::Json(Interaction { project_user, project_repo, type_ }): axum::Json<Interaction>,
) -> Result<(), AppError> {
    let user_id = db.get_user_id(github_id).await?;

    sqlx::query!(
        r#"
        INSERT INTO interactions (user_id, project_id, type)
        SELECT $1, p.id, $4
        FROM projects p
        INNER JOIN users u ON p.user_id = u.id
        WHERE u.username = $2
        AND p.repo_name = $3
        "#,
        user_id,
        project_user,
        project_repo,
        type_,
    )
    .execute(&*db)
    .await?;

    Ok(())    
}