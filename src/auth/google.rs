use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct GoogleJwks {
    keys: Vec<GoogleJwk>,
}

#[derive(Debug, Deserialize)]
struct GoogleJwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GoogleClaims {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub aud: String,
    pub iss: String,
    pub exp: usize,
}

const GOOGLE_CERTS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

/// Verifies a Google Sign-In ID token (the `credential` you get from Google
/// Identity Services on the frontend) and returns its claims.
///
/// This fetches Google's current signing keys on every call, which is fine
/// at low volume. If this becomes a hot path, cache the JWKS response with
/// a short TTL (Google rotates keys infrequently).
pub async fn verify_google_id_token(
    id_token: &str,
    expected_client_id: &str,
) -> anyhow::Result<GoogleClaims> {
    let header = decode_header(id_token)?;
    let kid = header.kid.ok_or_else(|| anyhow::anyhow!("token missing kid"))?;

    let jwks: GoogleJwks = reqwest::get(GOOGLE_CERTS_URL).await?.json().await?;
    let key = jwks
        .keys
        .into_iter()
        .find(|k| k.kid == kid)
        .ok_or_else(|| anyhow::anyhow!("no matching Google signing key"))?;

    let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[expected_client_id]);
    validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);

    let token_data = decode::<GoogleClaims>(id_token, &decoding_key, &validation)?;

    if !token_data.claims.email_verified {
        anyhow::bail!("Google email not verified");
    }

    Ok(token_data.claims)
}
