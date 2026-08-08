use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::extractors::AdminUser;
use crate::error::AppError;
use crate::AppState;

// ---------------------------------------------------------------------
// Dashboard stats — the "is the site actually working" numbers.
// ---------------------------------------------------------------------

#[derive(Serialize)]
pub struct AdminStats {
    pub total_users: i64,
    pub new_users_today: i64,
    pub new_users_this_week: i64,
    pub active_users_24h: i64,
    pub active_users_7d: i64,
    pub total_quizzes: i64,
    pub published_quizzes: i64,
    pub total_attempts: i64,
    pub attempts_today: i64,
    pub premium_users: i64,
}

pub async fn get_stats(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
) -> Result<Json<AdminStats>, AppError> {
    let row = sqlx::query_as::<_, (
        i64, i64, i64, i64, i64, i64, i64, i64, i64, i64,
    )>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM users) AS total_users,
            (SELECT COUNT(*) FROM users WHERE created_at >= date_trunc('day', now())) AS new_users_today,
            (SELECT COUNT(*) FROM users WHERE created_at >= now() - interval '7 days') AS new_users_this_week,
            (SELECT COUNT(DISTINCT user_id) FROM quiz_attempts WHERE completed_at >= now() - interval '24 hours') AS active_users_24h,
            (SELECT COUNT(DISTINCT user_id) FROM quiz_attempts WHERE completed_at >= now() - interval '7 days') AS active_users_7d,
            (SELECT COUNT(*) FROM quizzes) AS total_quizzes,
            (SELECT COUNT(*) FROM quizzes WHERE is_published = true) AS published_quizzes,
            (SELECT COUNT(*) FROM quiz_attempts) AS total_attempts,
            (SELECT COUNT(*) FROM quiz_attempts WHERE completed_at >= date_trunc('day', now())) AS attempts_today,
            (SELECT COUNT(*) FROM users WHERE subscription_tier = 'premium' AND (subscription_expires_at IS NULL OR subscription_expires_at > now())) AS premium_users
        "#,
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(AdminStats {
        total_users: row.0,
        new_users_today: row.1,
        new_users_this_week: row.2,
        active_users_24h: row.3,
        active_users_7d: row.4,
        total_quizzes: row.5,
        published_quizzes: row.6,
        total_attempts: row.7,
        attempts_today: row.8,
        premium_users: row.9,
    }))
}

// ---------------------------------------------------------------------
// Quiz list + delete — "remove the ones I hate"
// ---------------------------------------------------------------------

#[derive(Serialize, sqlx::FromRow)]
pub struct AdminQuizRow {
    pub id: Uuid,
    pub title: String,
    pub category: String,
    pub difficulty: String,
    pub is_published: bool,
    pub attempt_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

pub async fn list_quizzes_admin(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<AdminQuizRow>>, AppError> {
    let limit = p.per_page.unwrap_or(50).min(200);
    let offset = p.page.unwrap_or(0) * limit;

    let rows = sqlx::query_as::<_, AdminQuizRow>(
        r#"
        SELECT
            q.id,
            COALESCE(
                (SELECT title FROM quiz_translations WHERE quiz_id = q.id AND language_code = 'uz'),
                (SELECT title FROM quiz_translations WHERE quiz_id = q.id LIMIT 1),
                '(no title)'
            ) AS title,
            q.category,
            q.difficulty,
            q.is_published,
            (SELECT COUNT(*) FROM quiz_attempts WHERE quiz_id = q.id) AS attempt_count,
            q.created_at
        FROM quizzes q
        ORDER BY q.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

pub async fn delete_quiz(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(quiz_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM quizzes WHERE id = $1")
        .bind(quiz_id)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------
// User list — who's actually signed up
// ---------------------------------------------------------------------

#[derive(Serialize, sqlx::FromRow)]
pub struct AdminUserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub subscription_tier: String,
    pub created_at: DateTime<Utc>,
    pub attempt_count: i64,
}

pub async fn list_users(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<AdminUserRow>>, AppError> {
    let limit = p.per_page.unwrap_or(50).min(200);
    let offset = p.page.unwrap_or(0) * limit;

    let rows = sqlx::query_as::<_, AdminUserRow>(
        r#"
        SELECT
            u.id, u.email, u.display_name, u.role, u.subscription_tier, u.created_at,
            (SELECT COUNT(*) FROM quiz_attempts WHERE user_id = u.id) AS attempt_count
        FROM users u
        ORDER BY u.created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}

#[derive(Serialize)]
pub struct CreatedId {
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct QuizTranslationInput {
    pub language_code: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateQuizRequest {
    pub category: String,
    pub difficulty: String,
    pub translations: Vec<QuizTranslationInput>,
}

pub async fn create_quiz(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(payload): Json<CreateQuizRequest>,
) -> Result<Json<CreatedId>, AppError> {
    let mut tx = state.db.begin().await?;

    let quiz_id: Uuid = sqlx::query_scalar(
        "INSERT INTO quizzes (category, difficulty, created_by) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&payload.category)
    .bind(&payload.difficulty)
    .bind(admin.user_id)
    .fetch_one(&mut *tx)
    .await?;

    for t in &payload.translations {
        sqlx::query(
            "INSERT INTO quiz_translations (quiz_id, language_code, title, description)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(quiz_id)
        .bind(&t.language_code)
        .bind(&t.title)
        .bind(&t.description)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Json(CreatedId { id: quiz_id }))
}

#[derive(Deserialize)]
pub struct QuestionTranslationInput {
    pub language_code: String,
    pub question_text: String,
}

#[derive(Deserialize)]
pub struct AddQuestionRequest {
    pub order_index: i32,
    pub points: i32,
    pub translations: Vec<QuestionTranslationInput>,
}

pub async fn add_question(
    State(state): State<AppState>,
    Path(quiz_id): Path<Uuid>,
    AdminUser(_admin): AdminUser,
    Json(payload): Json<AddQuestionRequest>,
) -> Result<Json<CreatedId>, AppError> {
    let mut tx = state.db.begin().await?;

    let question_id: Uuid = sqlx::query_scalar(
        "INSERT INTO questions (quiz_id, order_index, points) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(quiz_id)
    .bind(payload.order_index)
    .bind(payload.points)
    .fetch_one(&mut *tx)
    .await?;

    for t in &payload.translations {
        sqlx::query(
            "INSERT INTO question_translations (question_id, language_code, question_text)
             VALUES ($1, $2, $3)",
        )
        .bind(question_id)
        .bind(&t.language_code)
        .bind(&t.question_text)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Json(CreatedId { id: question_id }))
}

#[derive(Deserialize)]
pub struct OptionTranslationInput {
    pub language_code: String,
    pub option_text: String,
}

#[derive(Deserialize)]
pub struct AddOptionRequest {
    pub order_index: i32,
    pub is_correct: bool,
    pub translations: Vec<OptionTranslationInput>,
}

pub async fn add_option(
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
    AdminUser(_admin): AdminUser,
    Json(payload): Json<AddOptionRequest>,
) -> Result<Json<CreatedId>, AppError> {
    let mut tx = state.db.begin().await?;

    let option_id: Uuid = sqlx::query_scalar(
        "INSERT INTO options (question_id, order_index, is_correct) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(question_id)
    .bind(payload.order_index)
    .bind(payload.is_correct)
    .fetch_one(&mut *tx)
    .await?;

    for t in &payload.translations {
        sqlx::query(
            "INSERT INTO option_translations (option_id, language_code, option_text)
             VALUES ($1, $2, $3)",
        )
        .bind(option_id)
        .bind(&t.language_code)
        .bind(&t.option_text)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Json(CreatedId { id: option_id }))
}

pub async fn publish_quiz(
    State(state): State<AppState>,
    Path(quiz_id): Path<Uuid>,
    AdminUser(_admin): AdminUser,
) -> Result<StatusCode, AppError> {
    sqlx::query("UPDATE quizzes SET is_published = true, updated_at = now() WHERE id = $1")
        .bind(quiz_id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}