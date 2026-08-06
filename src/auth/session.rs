use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    pub user_id: Uuid,
    pub role: String,
    pub exp: usize,
}

pub fn issue_session_token(user_id: Uuid, role: &str, secret: &str) -> anyhow::Result<String> {
    let exp = (Utc::now() + Duration::days(30)).timestamp() as usize;
    let claims = SessionClaims { user_id, role: role.to_string(), exp };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn verify_session_token(token: &str, secret: &str) -> anyhow::Result<SessionClaims> {
    let data = decode::<SessionClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(data.claims)
}
