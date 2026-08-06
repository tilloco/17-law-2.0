use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub google_client_id: String,
    pub session_jwt_secret: String,
    pub port: u16,
    /// Your deployed frontend URL, e.g. "https://17law.vercel.app".
    /// Local dev (http://localhost:5173) is always allowed alongside this.
    pub frontend_origin: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            google_client_id: env::var("GOOGLE_CLIENT_ID")
                .expect("GOOGLE_CLIENT_ID must be set"),
            session_jwt_secret: env::var("SESSION_JWT_SECRET")
                .expect("SESSION_JWT_SECRET must be set"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a number"),
            frontend_origin: env::var("FRONTEND_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
        }
    }
}
