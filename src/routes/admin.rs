use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::extractors::AdminUser;
use crate::error::AppError;
use crate::AppState;

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
