use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub google_id: String,
    pub email: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub subscription_tier: String,
    pub subscription_expires_at: Option<DateTime<Utc>>,
    pub preferred_language: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}
