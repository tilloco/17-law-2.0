pub mod admin;
pub mod auth;
pub mod quizzes;

use axum::http::HeaderValue;
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::AppState;

pub fn build_router(state: AppState) -> Router {
    // Cookies (used for the session) require an exact origin, not `Any`,
    // once credentials are involved. Local dev is always allowed; the
    // production frontend comes from FRONTEND_ORIGIN (see config.rs).
    let mut allowed_origins = vec!["http://localhost:5173".parse::<HeaderValue>().unwrap()];
    if let Ok(prod_origin) = state.config.frontend_origin.parse::<HeaderValue>() {
        if !allowed_origins.contains(&prod_origin) {
            allowed_origins.push(prod_origin);
        }
    }

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
.allow_methods([
    axum::http::Method::GET,
    axum::http::Method::POST,
    axum::http::Method::DELETE,
    axum::http::Method::OPTIONS,
])        .allow_headers([
    axum::http::header::AUTHORIZATION,
    axum::http::header::CONTENT_TYPE,
    axum::http::header::ACCEPT,
])
        .allow_credentials(true);

    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/auth/google", post(auth::google_sign_in))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout))
        .route("/quizzes", get(quizzes::list_quizzes))
        .route("/quizzes/:id", get(quizzes::get_quiz))
        .route("/quizzes/:id/attempt", post(quizzes::submit_attempt))
        .route(
            "/quizzes/:id/like",
            post(quizzes::like_quiz).delete(quizzes::unlike_quiz),
        )
        .route("/admin/quizzes", post(admin::create_quiz))
        .route("/admin/quizzes/:id/questions", post(admin::add_question))
        .route("/admin/questions/:id/options", post(admin::add_option))
        .route("/admin/quizzes/:id/publish", post(admin::publish_quiz))
        .with_state(state)
        .layer(cors)
}
