#![deny(clippy::all)]
#![deny(unused_must_use)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

// TODO: do this, and for the editor crate as well
// #![deny(warnings)]

use std::sync::LazyLock;

use anyhow::Context;
use api::api_router;
use auth::{TokenCache, middleware::redirect_auth_middleware};
use axum::{Router, extract::FromRef, middleware, routing::get};
use base64::{Engine, prelude::BASE64_STANDARD};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower_http::{
    add_extension::AddExtensionLayer,
    catch_panic::CatchPanicLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{db::DatabaseConnector, editor::session::EditorSessionManager, github::GithubClient};

mod api;
mod auth;
mod callback;
mod db;
mod editor;
mod error;
mod github;
mod lang;

const FRONT_PUBLIC: &str = "./front_end/dist";
// todo: check where this used to be used (probs delete it)
// const CLIENT_USER_AGENT: &str = "nea-website";
const GITHUB_APP_SLUG: &str = "nea-website";
const EDITOR_PATH: &str = "./editor/dist";
const SOCKET_ADDRESS: &str = "0.0.0.0:8080";

static CONFIG: LazyLock<Config> =
    LazyLock::new(|| Config::from_env().expect("failed to load env vars"));

struct Config {
    github_client_id: String,
    github_client_secret: String,
    database_url: String,
    aes_key: Vec<u8>,
}

impl Config {
    fn from_env() -> anyhow::Result<Self> {
        dotenv::dotenv().ok();

        Ok(Self {
            github_client_id: dotenv::var("GITHUB_CLIENT_ID")
                .context("missing GITHUB_CLIENT_ID")?,
            github_client_secret: dotenv::var("GITHUB_CLIENT_SECRET")
                .context("missing GITHUB_CLIENT_SECRET")?,
            database_url: dotenv::var("DATABASE_URL").context("missing DATABASE_URL")?,
            aes_key: BASE64_STANDARD
                .decode(dotenv::var("AES_KEY").context("missing AES_KEY")?)
                .context("invalid base64 AES_KEY")?,
        })
    }
}

#[derive(Clone, FromRef)]
struct AppState {
    pub client: GithubClient,
    pub db: DatabaseConnector,
    pub session_mgr: EditorSessionManager,
}

impl AppState {
    fn with_db(pool: PgPool) -> Self {
        Self {
            client: GithubClient::default(),
            db: DatabaseConnector::new(pool),
            session_mgr: EditorSessionManager::default(),
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from("nea_website=debug,tower_http=warn"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&CONFIG.database_url)
        .await
        .expect("failed to connect to database");

    let state = AppState::with_db(pool);

    let editor = Router::new()
        .fallback_service(
            ServeDir::new(EDITOR_PATH)
                .fallback(ServeFile::new(format!("{EDITOR_PATH}/index.html"))),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            redirect_auth_middleware,
        ));

    let app = Router::new()
        .nest("/api", api_router(state.clone()))
        .route("/callback", get(callback::github_callback))
        .nest("/editor", editor)
        .fallback_service(
            ServeDir::new(FRONT_PUBLIC)
                .fallback(ServeFile::new(format!("{FRONT_PUBLIC}/index.html"))),
        )
        .with_state(state)
        .layer(AddExtensionLayer::new(TokenCache::default()))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(SOCKET_ADDRESS).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
