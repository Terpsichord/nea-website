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

// Path to the compiled HTML/CSS/JS source files to serve when a request to the main front_end is receiveed
const FRONT_PUBLIC: &str = "./front_end/dist";
// todo: check where this used to be used (probs delete it)
// const CLIENT_USER_AGENT: &str = "nea-website";
// Registered name of the GitHub App for the website
const GITHUB_APP_SLUG: &str = "nea-website";
// Path to the compiled HTML/CSS/JS for the editor frontend
const EDITOR_PATH: &str = "./editor/dist";
// Socket on which the server listens for incoming requests (port 8080)
const SOCKET_ADDRESS: &str = "0.0.0.0:8080";

static CONFIG: LazyLock<Config> =
    LazyLock::new(|| Config::from_env().expect("failed to load env vars"));

// Config and settings for the app
struct Config {
    // ID that identiifes the GitHub App
    github_client_id: String,
    // Secret code for authenticating the GitHub App
    github_client_secret: String,
    // URL to describe how to access the PostgreSQL database
    database_url: String,
    // Private key used for AES encryption/decryption
    aes_key: Vec<u8>,
}

impl Config {
    // Loads `Config` from environment variables
    // Config is stored in a .env file so that confidential settings are not exposed publicly, e.g. through GitHub
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

// Shared state across all server endpoint handlers 
#[derive(Clone, FromRef)]
struct AppState {
    // Client to make GitHub API requests
    pub client: GithubClient,
    // Connection to Postgres Database
    pub db: DatabaseConnector,
    // Manages state of all editor sessions
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

// Main entry point function into the server
// This is where all initialisation for the web app occurs and where all of the frontends are served from
#[tokio::main]
async fn main() {
    // initialise logging to display debug/error messages from the app
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from("nea_website=debug,tower_http=warn"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // initialise connection to the database, using the DATABASE_URL in config
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&CONFIG.database_url)
        .await
        .expect("failed to connect to database");

    // initialise shared app state
    let state = AppState::with_db(pool);


    // describe routing behaviour for the /editor route
    let editor = Router::new()
        .fallback_service(
            // Serve the compiled editor files
            ServeDir::new(EDITOR_PATH)
                .fallback(ServeFile::new(format!("{EDITOR_PATH}/index.html"))),
        )
        // redirect to landing page if trying to access editor when unauthenticated
        .layer(middleware::from_fn_with_state(
            state.clone(),
            redirect_auth_middleware,
        ));

    // describe routing for whole app
    let app = Router::new()
        // user api_router for API requests
        .nest("/api", api_router(state.clone()))
        .route("/callback", get(callback::github_callback))
        .nest("/editor", editor)
        // serve the main frontend by default
        .fallback_service(
            ServeDir::new(FRONT_PUBLIC)
                .fallback(ServeFile::new(format!("{FRONT_PUBLIC}/index.html"))),
        )
        // add shared state
        .with_state(state)
        // add shared token cache
        .layer(AddExtensionLayer::new(TokenCache::default()))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http());

    // start listening for incoming TCP packets on the SOCKET_ADDRESS
    let listener = tokio::net::TcpListener::bind(SOCKET_ADDRESS).await.unwrap();

    // serve the HTTP requests using the routing for the app, described above 
    axum::serve(listener, app).await.unwrap();
}
