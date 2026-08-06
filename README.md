# 17 Law — backend

Rust + Axum + PostgreSQL. Google Sign-In, 3-language quiz content, admin-only
quiz management, likes, and scored attempts. Subscription fields are on
`users` but no payment processor is wired up yet — see the plan doc for that.

## Setup

1. Install Rust (rustup.rs) and have a Postgres instance running.
2. `createdb seventeen_law`
3. `cp .env.example .env` and fill in:
   - `DATABASE_URL` — your Postgres connection string
   - `GOOGLE_CLIENT_ID` — from a Google Cloud OAuth 2.0 Client ID (Credentials page, type "Web application")
   - `SESSION_JWT_SECRET` — any long random string (e.g. `openssl rand -hex 32`)
4. `cargo run` — this connects to Postgres and runs the migration in
   `migrations/0001_init.sql` automatically on startup, then starts the server.

## Getting your first admin user

There's no signup flow for admin — sign in once with Google (which creates
your user row with the default `'user'` role), then promote yourself directly
in the database:

```sql
UPDATE users SET role = 'admin' WHERE email = 'you@example.com';
```

## Frontend: sending the Google token

Using Google Identity Services directly (works with any frontend framework):

```html
<script src="https://accounts.google.com/gsi/client" async defer></script>
<div id="g_id_onload"
     data-client_id="YOUR_GOOGLE_CLIENT_ID"
     data-callback="handleCredentialResponse">
</div>
<div class="g_id_signin" data-type="standard"></div>

<script>
async function handleCredentialResponse(response) {
  await fetch("http://localhost:8080/auth/google", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    credentials: "include", // needed so the session cookie gets set
    body: JSON.stringify({ id_token: response.credential }),
  });
  // session cookie is now set — reload or update your app's auth state
}
</script>
```

Every later request that needs auth (attempts, likes, admin routes) needs
`credentials: "include"` on the fetch call so the session cookie is sent.

## Things to double-check on your machine

I wrote this without a live compiler in front of me, so a couple of spots are
version-sensitive and worth a first `cargo build` before you trust them:

- **`Option<AuthUser>` in `list_quizzes`** (`src/routes/quizzes.rs`) relies on
  axum's optional-extractor support. If it doesn't compile, replace it with
  a manual cookie check instead of the extractor.
- **`async fn` directly in the `FromRequestParts` impls** (`src/auth/extractors.rs`)
  targets recent axum 0.7. Older axum wants `#[axum::async_trait]` above each
  `impl` block — add it if the compiler asks for it.
- **`Cookie::build(...)`** syntax (`src/routes/auth.rs`) matches `cookie` crate
  0.18's builder. If `axum-extra` pulls in a different `cookie` version, the
  builder API may shift slightly.
- `sqlx::query_as::<_, T>(...)` (the function form, not the `query!` macro) is
  used everywhere on purpose — it's checked at runtime, not compile time, so
  you don't need a live `DATABASE_URL` just to `cargo build`. Once things are
  stable you may want to migrate hot-path queries to `query_as!` for
  compile-time SQL checking.

## What's not built yet

- Frontend (React/Vue/whatever you pick — the API above is framework-agnostic)
- Payment integration for subscriptions (Payme/Click, when you're ready)
- Rate limiting / abuse protection on public endpoints
