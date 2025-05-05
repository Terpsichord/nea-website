use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    middleware,
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};

use crate::{
    error::AppError,
    middlewares::auth::{auth_middleware, optional_auth_middleware, AuthUser},
    AppState,
};

use super::user::ProjectInfo;

pub fn project_router(state: AppState) -> Router<AppState> {
    let auth = Router::new().route("/project/open/{project_id}", get(open_project));

    Router::new()
        .route("/projects", get(get_project_list))
        .merge(auth)
        .nest(
            "/project/{username}/{repo_name}",
            project_page_router(state),
        )
}

fn project_page_router(state: AppState) -> Router<AppState> {
    let auth = Router::new()
        .route("/liked", get(get_liked))
        .route("/like", post(like))
        .route("/unlike", post(unlike))
        .route("/comment", post(post_comment))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    let optional_auth =
        Router::new()
            .route("/", get(get_project))
            .layer(middleware::from_fn_with_state(
                state,
                optional_auth_middleware,
            ));

    Router::new()
        .merge(auth)
        .merge(optional_auth)
        .route("/comments", get(get_comments))
}

#[derive(Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
struct ProjectResponse {
    #[serde(flatten)]
    info: ProjectInfo,
    github_url: String,
    upload_time: NaiveDateTime,
    public: bool,
}

async fn get_project(
    Path((username, repo_name)): Path<(String, String)>,
    Extension(auth_user): Extension<Option<AuthUser>>,
    State(db): State<PgPool>,
) -> Result<Json<ProjectResponse>, AppError> {
    let authorized = match auth_user {
        Some(AuthUser { github_id }) => {
            sqlx::query_scalar!("SELECT username FROM users WHERE github_id = $1", github_id)
                .fetch_one(&db)
                .await?
                == username
        }
        None => false,
    };

    let project = sqlx::query_as!(
        ProjectResponse,
        r#"
        SELECT 
            (p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags, pi.like_count) as "info!: ProjectInfo",
            pi.github_url as "github_url!",
            p.upload_time,
            p.public
        FROM projects p
        INNER JOIN project_info pi ON pi.id = p.id
        WHERE pi.username = $1
        AND p.repo_name = $2
        AND (p.public OR $3)
        "#,
        username,
        repo_name,
        authorized
    ).fetch_one(&db).await?;

    Ok(Json(project))
}

async fn get_project_list(
    State(db): State<PgPool>,
) -> Result<Json<Vec<ProjectResponse>>, AppError> {
    let projects = sqlx::query_as!(ProjectResponse, r#"
        SELECT
            (p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags, pi.like_count) as "info!: ProjectInfo",
            pi.github_url as "github_url!",
            p.upload_time,
            p.public
        FROM projects p
        INNER JOIN project_info pi ON pi.id = p.id
        WHERE p.public
        ORDER BY upload_time DESC
    "#).fetch_all(&db).await?;

    Ok(Json(projects))
}

async fn get_liked(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<PgPool>,
    Extension(AuthUser { github_id }): Extension<AuthUser>,
) -> Result<Json<bool>, AppError> {
    let liked = sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM likes l
            INNER JOIN projects p ON p.id = l.project_id
            INNER JOIN users lu ON lu.id = l.user_id
            INNER JOIN users pu ON pu.id = p.user_id
            WHERE lu.github_id = $1
            AND pu.username = $2
            AND p.repo_name = $3
        ) as "liked!"
        "#,
        github_id,
        username,
        repo_name
    )
    .fetch_one(&db)
    .await?;

    Ok(Json(liked))
}

async fn like(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<PgPool>,
    Extension(AuthUser { github_id }): Extension<AuthUser>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        INSERT INTO likes (user_id, project_id)
        SELECT 
        (SELECT id FROM users WHERE github_id = $1),
        (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $2
            AND p.repo_name = $3
        )
        "#,
        github_id,
        username,
        repo_name
    )
    .execute(&db)
    .await?;

    Ok(())
}

async fn unlike(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<PgPool>,
    Extension(AuthUser { github_id }): Extension<AuthUser>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        DELETE FROM likes
        WHERE user_id = (SELECT id FROM users WHERE github_id = $1)
        AND project_id = 
        (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $2
            AND p.repo_name = $3
        )
        "#,
        github_id,
        username,
        repo_name
    )
    .execute(&db)
    .await?;

    Ok(())
}

#[derive(Deserialize)]
struct PostCommentBody {
    contents: String,
    parent_id: Option<i32>,
}

async fn post_comment(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<PgPool>,
    Extension(AuthUser { github_id }): Extension<AuthUser>,
    Json(PostCommentBody { contents, parent_id }): Json<PostCommentBody>,
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
    .execute(&db)
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
    children: Vec<Comment>,
    #[serde(skip)]
    parent_id: Option<i32>,
}

async fn get_comments(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<PgPool>,
) -> Result<Json<Vec<Comment>>, AppError> {
    let mut comments = sqlx::query_as!(
        Comment,
        r#"
        SELECT
            c.id,
            c.parent_id,
            (u.username, u.picture_url) as "user!: InlineUser",
            c.contents,
            array[]::integer[] as "children!: Vec<Comment>"
        FROM comments c
        INNER JOIN users u ON u.id = c.user_id
        WHERE c.project_id = (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $1
            AND p.repo_name = $2
        )
        ORDER BY c.upload_time DESC
        "#,
        username,
        repo_name,
    )
    .fetch_all(&db)
    .await?;

    // let mut comment_map = HashMap::new();

    // for comment in comments {
    //     comment_map.insert(comment.id, comment);
    // }

    // let ids: Vec<_> = comment_map.keys().copied().collect();
    // for id in ids {
    //     if let Some(parent_id) = comment_map[&id].parent_id {
    //         let comment = comment_map[&id].clone();
    //         comment_map
    //             .get_mut(&parent_id)
    //             // can unwrap here as this is guaranteed by foreign key constraints in the database
    //             .unwrap()
    //             .children
    //             .push(comment);
    //     }
    // }
    // 
    // let root_comments = comment_map
    //     .into_values()
    //     .filter(|com| com.parent_id.is_none())
    //     .collect();

    // FIXME: this seems to work at the moment, but, make this better and more optimised

    let mut roots = vec![];
    for comment in comments.iter() {
        if comment.parent_id.is_none() {
           roots.push(comment.clone());
        }
    }
    for root in &mut roots {
        root.children = get_comment_replies(root.id, &mut comments);
    }

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

async fn open_project(
    Path(project_id): Path<String>,
    State(db): State<PgPool>,
    Extension(AuthUser { github_id }): Extension<AuthUser>,
) -> Result<Json<Value>, AppError> {
    let github_url = sqlx::query_scalar!(
        r#"
        SELECT github_url
        FROM projects
        WHERE editor_id = $1
        AND user_id = (
            SELECT id
            FROM users
            WHERE github_id = $2
        )
        "#,
        project_id,
        github_id,
    )
    .fetch_one(&db)
    .await?;

    // FIXME: this is just to test interop between editor and backend
    Ok(Json(json!({ "github_url": github_url })))
}
