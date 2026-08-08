pub mod admin;
pub mod auth;
pub mod quizzes;

use axum::http::HeaderValue;
use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

use crate::AppState;

pub fn build_router(state: AppState) -> Router {
    // Cookies (used for the session) require exact origins, not `Any`,
    // once credentials are involved. Add every real frontend origin here —
    // local dev AND production. Missing one means that origin's requests
    // get silently blocked by the browser, not a helpful error.
    let allowed_origins = [
        "http://localhost:5173",
        "https://17law-frontend-psi.vercel.app",
        // "https://huquq17.com",       // uncomment once the custom domain is live
        // "https://www.huquq17.com",
    ];
    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origins
                .iter()
                .map(|o| o.parse::<HeaderValue>().unwrap())
                .collect::<Vec<_>>(),
        )
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::DELETE,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE])
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
        .route("/admin/stats", get(admin::get_stats))
        .route("/admin/users", get(admin::list_users))
        .route(
            "/admin/quizzes",
            get(admin::list_quizzes_admin).post(admin::create_quiz),
        )
        .route("/admin/quizzes/:id", axum::routing::delete(admin::delete_quiz))
        .route("/admin/quizzes/:id/questions", post(admin::add_question))
        .route("/admin/questions/:id/options", post(admin::add_option))
        .route("/admin/quizzes/:id/publish", post(admin::publish_quiz))
        .with_state(state)
        .layer(cors)
}