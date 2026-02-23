use std::cmp::Reverse;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    AppState,
    auth::middleware::{AuthUser, auth_middleware},
    db::DatabaseConnector,
    error::AppError,
};

pub fn comment_router(state: AppState) -> Router<AppState> {
    let auth = Router::new()
        .route("/comment/{username}/{repo_name}", post(post_comment))
        .layer(middleware::from_fn_with_state(state, auth_middleware));

    Router::new()
        .route("/comment/{username}/{repo_name}", get(get_comments))
        .merge(auth)
}

#[derive(Deserialize)]
struct PostCommentBody {
    contents: String,
    parent_id: Option<i32>,
}

#[instrument(skip(db))]
async fn post_comment(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<DatabaseConnector>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
    Json(PostCommentBody {
        contents,
        parent_id,
    }): Json<PostCommentBody>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO comments (contents, user_id, project_id, parent_id)
        SELECT 
            $1, 
            (SELECT id FROM users WHERE github_id = $2),
            (
                SELECT p.id
                FROM projects p
                INNER JOIN users u ON u.id = p.user_id
                WHERE u.username = $3
                AND p.repo_name = $4
            ),
            $5
        "#,
        contents,
        github_id,
        username,
        repo_name,
        parent_id
    )
    .execute(&*db)
    .await?;

    Ok(())
}

#[derive(Clone, Debug, Serialize, sqlx::Type)]
#[serde(rename_all = "camelCase")]
struct InlineUser {
    username: String,
    picture_url: String,
}

#[derive(Clone, Debug, Serialize, sqlx::Type)]
#[serde(rename_all = "camelCase")]
struct Comment {
    id: i32,
    user: InlineUser,
    contents: String,
    children: Vec<Self>,
    #[serde(skip)]
    parent_id: Option<i32>,
    #[serde(skip)]
    upload_time: DateTime<Utc>,
}

#[instrument(skip(db))]
async fn get_comments(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<DatabaseConnector>,
) -> Result<Json<Vec<Comment>>, AppError> {
    let mut comments = sqlx::query_as!(
        Comment,
        r#"
        SELECT
            c.id,
            c.parent_id,
            (u.username, u.picture_url) as "user!: InlineUser",
            c.contents,
            array[]::integer[] as "children!: Vec<Comment>",
            c.upload_time
        FROM comments c
        INNER JOIN users u ON u.id = c.user_id
        WHERE c.project_id = (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $1
            AND p.repo_name = $2
        )
        ORDER BY c.upload_time
        "#,
        username,
        repo_name,
    )
    .fetch_all(&*db)
    .await?;

    let mut roots = vec![];
    for comment in &comments {
        if comment.parent_id.is_none() {
            roots.push(comment.clone());
        }
    }
    for root in &mut roots {
        root.children = get_comment_replies(root.id, &mut comments);
    }

    roots.sort_by_key(|root| Reverse(root.upload_time));

    Ok(Json(roots))
}

fn get_comment_replies(id: i32, comments: &mut [Comment]) -> Vec<Comment> {
    let mut children = vec![];
    for i in 0..comments.len() {
        if comments[i].parent_id == Some(id) {
            comments[i].children = get_comment_replies(comments[i].id, comments);
            children.push(comments[i].clone());
        }
    }

    children
}
