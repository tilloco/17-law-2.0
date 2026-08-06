mod auth;
mod config;
mod db;
mod error;
mod models;
mod routes;

use std::sync::Arc;

use axum::extract::FromRef;
use sqlx::PgPool;

use config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Config::from_env();
    let pool = db::init_pool(&config.database_url).await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let port = config.port;
    let state = AppState { db: pool, config: Arc::new(config) };
    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind port");

    tracing::info!("17 Law backend listening on port {port}");
    axum::serve(listener, app).await.expect("server error");
}
