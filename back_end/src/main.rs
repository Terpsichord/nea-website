#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use std::{iter, sync::LazyLock};

use anyhow::Context;
use api::api_router;
use axum::{extract::FromRef, http::HeaderValue, middleware, routing::get, Router};
use base64::{prelude::BASE64_STANDARD, Engine};
use middlewares::auth::{redirect_auth_middleware, SharedTokenIds};
use reqwest::header::USER_AGENT;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tower_http::{
    add_extension::AddExtensionLayer,
    catch_panic::CatchPanicLayer,
    services::{ServeDir, ServeFile},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod callback;
mod crypto;
mod db;
mod error;
mod middlewares;
mod user;

const FRONT_PUBLIC: &str = "./front_end/dist";
const CLIENT_USER_AGENT: &str = "nea-website";
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
    pub client: reqwest::Client,
    pub db: PgPool,
}

impl AppState {
    fn with_db(pool: PgPool) -> Self {
        Self {
            client: reqwest::Client::builder()
                .default_headers(
                    iter::once((USER_AGENT, HeaderValue::from_static(CLIENT_USER_AGENT))).collect(),
                )
                .build()
                .unwrap(),
            db: pool,
        }
    }
}

#[tokio::main]
async fn main() {
    // start tracing - level set by either RUST_LOG env variable or defaults to debug
    // TODO: check i've configured this the way i want it
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "nea_website=debug".into()),
        ))
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
        .layer(AddExtensionLayer::new(SharedTokenIds::default()))
        .layer(CatchPanicLayer::new());

    let listener = tokio::net::TcpListener::bind(SOCKET_ADDRESS).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
