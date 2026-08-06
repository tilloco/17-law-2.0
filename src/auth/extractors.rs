use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum_extra::extract::CookieJar;

use crate::auth::session::verify_session_token;
use crate::error::AppError;
use crate::AppState;

pub struct AuthUser {
    pub user_id: uuid::Uuid,
    pub role: String,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let jar = CookieJar::from_headers(&parts.headers);
        let cookie = jar.get("session").ok_or(AppError::Unauthorized)?;

        let claims = verify_session_token(cookie.value(), &app_state.config.session_jwt_secret)
            .map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser { user_id: claims.user_id, role: claims.role })
    }
}

/// Wraps AuthUser and additionally requires role == "admin".
/// Since this is a single-admin app, a role check is all that's needed —
/// no separate permissions table.
pub struct AdminUser(pub AuthUser);

#[axum::async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != "admin" {
            return Err(AppError::Forbidden);
        }
        Ok(AdminUser(user))
    }
}
