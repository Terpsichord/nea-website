use axum::{
    Extension, Json, Router,
    extract::{Path, State, WebSocketUpgrade, ws::WebSocket},
    middleware,
    response::Response,
    routing::{get, post, put},
};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::{
    AppState,
    api::{ProjectResponse, search},
    auth::middleware::{AuthUser, auth_middleware, optional_auth_middleware},
    db::{DatabaseConnector, NewProject},
    editor::websocket::WebSocketHandler,
    error::AppError,
    github::{CreateRepoResponse, access_tokens::WithTokens},
    lang::ProjectLang,
};

pub fn project_router(state: AppState) -> Router<AppState> {
    let auth = Router::new()
        .route("/project/{username}/{repo_name}/open", get(open_project))
        .route("/project/new", post(new_project))
        .route("/project/{username}/{repo_name}/remix", post(remix_project))
        .route("/project/{username}/{repo_name}", put(update_project))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .route("/project/search", get(search::search_projects))
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

    Router::new().merge(auth).merge(optional_auth)
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
struct NewProjectBody {
    title: String,
    lang: ProjectLang,
    private: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewProjectResponse {
    username: String,
    repo_name: String,
}

#[instrument(skip(db, client, access, refresh))]
async fn new_project(
    State(AppState { db, client, .. }): State<AppState>,
    Extension(AuthUser {
        github_id,
        access_token: access,
        refresh_token: refresh,
    }): Extension<AuthUser>,
    Json(NewProjectBody {
        title,
        lang,
        private,
    }): Json<NewProjectBody>,
) -> Result<Json<NewProjectResponse>, AppError> {
    let mut access_token = &*access;
    let mut refresh_token = &*refresh;

    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE github_id = $1", github_id)
        .fetch_one(&*db)
        .await?;
    info!("user_id: {}", user_id);

    if db.project_exists(user_id, &title).await? {
        return Err(AppError::ProjectExists);
    }

    let username = sqlx::query_scalar!("SELECT username FROM users WHERE id = $1", user_id)
        .fetch_one(&*db)
        .await?;

    // create the github repo for the project
    let WithTokens(
        CreateRepoResponse {
            repo_name,
            already_exists,
        },
        mut tokens,
    ) = client
        .create_repo(
            access_token,
            refresh_token,
            &username,
            &title,
            lang,
            private,
        )
        .await?;
    info!("repo_name: {}", repo_name);

    if let Some(ref tokens) = tokens {
        (access_token, refresh_token) = tokens.unencrypted();
    }

    // if the repo already exists, get the readme
    let readme = if already_exists {
        let WithTokens(readme, new_tokens) = client
            .get_readme(access_token, refresh_token, &username, &repo_name)
            .await?;
        info!("readme: {}", repo_name);

        tokens = new_tokens.or(tokens);
        if let Some(ref tokens) = tokens {
            (access_token, refresh_token) = tokens.unencrypted();
        }

        Some(readme)
    } else {
        None
    }
    .unwrap_or_default();

    let new_project = NewProject {
        title,
        lang,
        user_id,
        repo_name,
        readme,
        public: !private,
        tags: vec![],
    };

    db.add_project(&new_project).await?;

    // FIXME: this function should return the token cookie headers
    // i'm not sure what the return type should be
    Ok(Json(NewProjectResponse {
        username,
        repo_name: new_project.repo_name,
    }))
}

#[instrument(skip(db, client, access_token, refresh_token))]
async fn remix_project(
    Path((username, repo_name)): Path<(String, String)>,
    State(AppState { db, client, .. }): State<AppState>,
    Extension(AuthUser {
        github_id,
        access_token,
        refresh_token,
    }): Extension<AuthUser>,
) -> Result<Json<NewProjectResponse>, AppError> {
    info!("remixing");
    let project = db.get_project(&username, &repo_name, None, false).await?;
    info!("found project {}", project.info.title);

    let user_id = sqlx::query_scalar!("SELECT id FROM users WHERE github_id = $1", github_id)
        .fetch_one(&*db)
        .await?;
    info!("user_id: {}", user_id);

    if db.project_exists(user_id, &project.info.title).await? {
        return Err(AppError::ProjectExists);
    }

    // create a fork of the github repo for the project
    let WithTokens((), tokens) = client
        .fork_repo(&access_token, &refresh_token, &username, &repo_name)
        .await?;
    info!("repo_name: {}", repo_name);

    let tags = sqlx::query_scalar!(
        r#"
        SELECT tag
        FROM project_tags
        WHERE project_id = $1
        "#,
        project.id
    )
    .fetch_all(&*db)
    .await?;

    let new_project = NewProject {
        title: project.info.title,
        repo_name,
        lang: project.lang,
        user_id,
        readme: project.info.readme,
        public: true,
        tags,
    };

    db.add_project(&new_project).await?;

    let new_username = sqlx::query_scalar!("SELECT username FROM users WHERE id = $1", user_id)
        .fetch_one(&*db)
        .await?;

    Ok(Json(NewProjectResponse {
        username: new_username,
        repo_name: new_project.repo_name,
    }))
}

#[derive(Deserialize)]
struct UpdateProjectBody {
    title: String,
    private: bool,
    tags: Vec<String>,
}

async fn update_project(
    Path((username, repo_name)): Path<(String, String)>,
    State(AppState { db, .. }): State<AppState>,
    Json(UpdateProjectBody {
        title,
        private,
        tags,
    }): Json<UpdateProjectBody>,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        UPDATE projects
        SET title = $1, public = $2
        WHERE id = 
        (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $3
            AND p.repo_name = $4
        )
        "#,
        title,
        !private,
        username,
        repo_name,
    )
    .execute(&*db)
    .await?;

    // remove all old tags
    sqlx::query!(
        r#"
        DELETE FROM project_tags
        WHERE project_id = 
        (
            SELECT p.id
            FROM projects p
            INNER JOIN users u ON u.id = p.user_id
            WHERE u.username = $1
            AND p.repo_name = $2
        )
        "#,
        username,
        repo_name
    )
    .execute(&*db)
    .await?;

    // and add new tags
    sqlx::query!(
        r#"
        INSERT INTO project_tags (project_id, tag)
        SELECT p.id, UNNEST($1::text[])
        FROM projects p
        INNER JOIN users u ON u.id = p.user_id
        WHERE u.username = $2
        AND p.repo_name = $3
        "#,
        &tags,
        username,
        repo_name,
    )
    .execute(&*db)
    .await?;

    Ok(())
}

#[instrument(skip(db, ws, session_mgr, access_token, refresh_token))]
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

    // todo: return these tokens
    let WithTokens(container_id, _) = session_mgr
        .open(
            project.user_id,
            project.id,
            &username,
            &repo_name,
            project.lang,
            &access_token,
            &refresh_token,
        )
        .await?;

    // let code = session_mgr.create_code(project.user_id);

    Ok(ws.on_upgrade(move |ws| handle_editor_ws(ws, container_id)))
}

async fn handle_editor_ws(ws: WebSocket, container_id: String) {
    let mut handler = WebSocketHandler::new(container_id).expect("TODO");

    handler.handle(ws).await;
}
