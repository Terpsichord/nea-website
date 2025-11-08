use std::cmp::Reverse;

use axum::{
    Extension, Json, Router,
    extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    middleware,
    response::Response,
    routing::{get, post},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::instrument;

use crate::{
    AppState,
    api::ProjectResponse,
    auth::{
        SharedTokenInfo,
        middleware::{AuthUser, auth_middleware, optional_auth_middleware},
    },
    db::DatabaseConnector,
    error::AppError,
};

use super::ProjectInfo;

pub fn project_router(state: AppState) -> Router<AppState> {
    let auth = Router::new()
        .route("/project/open/{username}/{repo_name}", get(open_project))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

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

#[instrument(skip(db))]
async fn get_project(
    Path((username, repo_name)): Path<(String, String)>,
    Extension(auth_user): Extension<Option<AuthUser>>,
    State(db): State<DatabaseConnector>,
) -> Result<Json<ProjectResponse>, AppError> {
    let project = db
        .get_project(
            &username,
            &repo_name,
            auth_user.map(|user| user.github_id),
            false,
        )
        .await?;

    Ok(Json(project.into()))
}

#[instrument(skip(db))]
async fn get_project_list(
    State(db): State<DatabaseConnector>,
) -> Result<Json<Vec<ProjectResponse>>, AppError> {
    let projects = sqlx::query_as!(ProjectResponse, r#"
        SELECT
            (p.title, pi.username, pi.picture_url, p.repo_name, p.readme, pi.tags, pi.like_count) as "info!: ProjectInfo",
            pi.github_url as "github_url!",
            p.upload_time,
            p.public,
            false as "owned!"
        FROM projects p
        INNER JOIN project_info pi ON pi.id = p.id
        WHERE p.public
        ORDER BY upload_time DESC
    "#).fetch_all(&*db).await?;

    Ok(Json(projects))
}

#[instrument(skip(db))]
async fn get_liked(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<DatabaseConnector>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
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
    .fetch_one(&*db)
    .await?;

    Ok(Json(liked))
}

async fn like(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<DatabaseConnector>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
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
    .execute(&*db)
    .await?;

    Ok(())
}

#[instrument(skip(db))]
async fn unlike(
    Path((username, repo_name)): Path<(String, String)>,
    State(db): State<DatabaseConnector>,
    Extension(AuthUser { github_id, .. }): Extension<AuthUser>,
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
    .execute(&*db)
    .await?;

    Ok(())
}

#[derive(Deserialize)]
struct PostCommentBody {
    contents: String,
    parent_id: Option<i32>,
}

// TODO: change this to be on a /comment/ endpoint like in your API diagram
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
    children: Vec<Comment>,
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

#[instrument(skip(db, session_mgr, access_token, refresh_token))]
async fn open_project(
    Path((username, repo_name)): Path<(String, String)>,
    State(AppState {
        db, session_mgr, ..
    }): State<AppState>,
    ws: WebSocketUpgrade,
    Extension(AuthUser {
        github_id,
        access_token,
        refresh_token,
    }): Extension<AuthUser>,
) -> Result<Response, AppError> {
    let project = db
        .get_project(&username, &repo_name, Some(github_id), true)
        .await?;

    session_mgr
        .open(
            project.user_id,
            project.id,
            &username,
            &repo_name,
            &access_token,
            &refresh_token,
        )
        .await?;

    // let code = session_mgr.create_code(project.user_id);

    Ok(ws.on_upgrade(handle_editor_ws))
}

async fn handle_editor_ws(ws: WebSocket) {
}

// async fn connect_session(
//     Path((username, repo_name, code)): Path<(String, String, String)>,
//     State(AppState {
//         db, session_mgr, ..
//     }): State<AppState>,
//     ws: WebSocketUpgrade,
// ) -> Result<Response, AppError> {
//     let user_id = sqlx::query_scalar!(
//         r#"
//         SELECT id
//         FROM users
//         WHERE username = $1
//         "#,
//         username
//     )
//     .execute(&*db)
//     .fetch_one()
//     .await?;

//     Ok(ws.on_upgrade(||))
// }
