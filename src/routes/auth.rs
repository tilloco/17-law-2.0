use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use serde::Deserialize;

use crate::auth::extractors::AuthUser;
use crate::auth::google::verify_google_id_token;
use crate::auth::session::issue_session_token;
use crate::error::AppError;
use crate::models::User;
use crate::AppState;

#[derive(Deserialize)]
pub struct GoogleSignInRequest {
    /// The `credential` string Google Identity Services hands you on the frontend.
    pub id_token: String,
}

pub async fn google_sign_in(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<GoogleSignInRequest>,
) -> Result<impl IntoResponse, AppError> {
    let claims = verify_google_id_token(&payload.id_token, &state.config.google_client_id)
        .await
        .map_err(|_| AppError::Unauthorized)?;

    // Upsert: first sign-in creates the user, later sign-ins just refresh their profile.
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (google_id, email, display_name, avatar_url)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (google_id) DO UPDATE
            SET email = EXCLUDED.email,
                display_name = EXCLUDED.display_name,
                avatar_url = EXCLUDED.avatar_url
        RETURNING *
        "#,
    )
    .bind(&claims.sub)
    .bind(&claims.email)
    .bind(claims.name.clone().unwrap_or_else(|| claims.email.clone()))
    .bind(&claims.picture)
    .fetch_one(&state.db)
    .await?;

    let token = issue_session_token(user.id, &user.role, &state.config.session_jwt_secret)
        .map_err(AppError::Internal)?;

    let cookie = Cookie::build(("session", token))
        .http_only(true)
        .secure(true) // requires HTTPS — Render/Vercel give you this by default
        .same_site(SameSite::None) // frontend and backend are on different domains
        .path("/")
        .build();

    Ok((jar.add(cookie), Json(user)))
}

/// Lets the frontend restore login state on page load — it only has the
/// httpOnly session cookie, not the user's data, so it needs to ask.
pub async fn me(user: AuthUser, State(state): State<AppState>) -> Result<Json<User>, AppError> {
    let row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user.user_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(row))
}

pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    (jar.remove(Cookie::from("session")), StatusCode::NO_CONTENT)
}
