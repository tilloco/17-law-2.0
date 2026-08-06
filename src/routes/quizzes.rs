use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::extractors::AuthUser;
use crate::error::AppError;
use crate::models::{OptionView, QuestionView, QuizDetail, QuizSummary};
use crate::AppState;

#[derive(Deserialize)]
pub struct LangParam {
    #[serde(default = "default_lang")]
    pub lang: String,
}

fn default_lang() -> String {
    "uz".to_string()
}

#[derive(sqlx::FromRow)]
struct QuizSummaryRow {
    id: Uuid,
    title: String,
    description: Option<String>,
    category: String,
    difficulty: String,
    like_count: i64,
    liked_by_me: bool,
}

// `user` here is optional: NOTE this relies on axum's blanket `Option<T>`
// support for custom extractors (stable in recent axum 0.7). If your
// version rejects it, swap to reading the cookie manually here instead.
pub async fn list_quizzes(
    State(state): State<AppState>,
    Query(params): Query<LangParam>,
    user: Option<AuthUser>,
) -> Result<Json<Vec<QuizSummary>>, AppError> {
    let user_id = user.map(|u| u.user_id);

    let rows = sqlx::query_as::<_, QuizSummaryRow>(
        r#"
        SELECT
            q.id,
            qt.title,
            qt.description,
            q.category,
            q.difficulty,
            COUNT(DISTINCT ql.user_id) AS like_count,
            EXISTS (
                SELECT 1 FROM quiz_likes WHERE quiz_id = q.id AND user_id = $2
            ) AS liked_by_me
        FROM quizzes q
        JOIN quiz_translations qt ON qt.quiz_id = q.id AND qt.language_code = $1
        LEFT JOIN quiz_likes ql ON ql.quiz_id = q.id
        WHERE q.is_published = true
        GROUP BY q.id, qt.title, qt.description, q.category, q.difficulty
        ORDER BY q.created_at DESC
        "#,
    )
    .bind(&params.lang)
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;

    let quizzes = rows
        .into_iter()
        .map(|r| QuizSummary {
            id: r.id,
            title: r.title,
            description: r.description,
            category: r.category,
            difficulty: r.difficulty,
            like_count: r.like_count,
            liked_by_me: r.liked_by_me,
        })
        .collect();

    Ok(Json(quizzes))
}

pub async fn get_quiz(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<LangParam>,
) -> Result<Json<QuizDetail>, AppError> {
    #[derive(sqlx::FromRow)]
    struct QuizRow {
        id: Uuid,
        title: String,
        description: Option<String>,
        category: String,
        difficulty: String,
    }

    let quiz = sqlx::query_as::<_, QuizRow>(
        r#"
        SELECT q.id, qt.title, qt.description, q.category, q.difficulty
        FROM quizzes q
        JOIN quiz_translations qt ON qt.quiz_id = q.id AND qt.language_code = $2
        WHERE q.id = $1 AND q.is_published = true
        "#,
    )
    .bind(id)
    .bind(&params.lang)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    #[derive(sqlx::FromRow)]
    struct QuestionRow {
        id: Uuid,
        text: String,
    }

    let question_rows = sqlx::query_as::<_, QuestionRow>(
        r#"
        SELECT qq.id, qqt.question_text AS text
        FROM questions qq
        JOIN question_translations qqt
            ON qqt.question_id = qq.id AND qqt.language_code = $2
        WHERE qq.quiz_id = $1
        ORDER BY qq.order_index
        "#,
    )
    .bind(id)
    .bind(&params.lang)
    .fetch_all(&state.db)
    .await?;

    #[derive(sqlx::FromRow)]
    struct OptionRow {
        id: Uuid,
        question_id: Uuid,
        text: String,
    }

    let option_rows = sqlx::query_as::<_, OptionRow>(
        r#"
        SELECT o.id, o.question_id, ot.option_text AS text
        FROM options o
        JOIN questions qq ON qq.id = o.question_id
        JOIN option_translations ot ON ot.option_id = o.id AND ot.language_code = $2
        WHERE qq.quiz_id = $1
        ORDER BY o.order_index
        "#,
    )
    .bind(id)
    .bind(&params.lang)
    .fetch_all(&state.db)
    .await?;

    let questions = question_rows
        .into_iter()
        .map(|q| QuestionView {
            options: option_rows
                .iter()
                .filter(|o| o.question_id == q.id)
                .map(|o| OptionView { id: o.id, text: o.text.clone() })
                .collect(),
            id: q.id,
            text: q.text,
        })
        .collect();

    Ok(Json(QuizDetail {
        id: quiz.id,
        title: quiz.title,
        description: quiz.description,
        category: quiz.category,
        difficulty: quiz.difficulty,
        questions,
    }))
}

#[derive(Deserialize)]
pub struct SubmitAttemptRequest {
    pub answers: Vec<AnswerSubmission>,
}

#[derive(Deserialize)]
pub struct AnswerSubmission {
    pub question_id: Uuid,
    pub option_id: Uuid,
}

#[derive(Serialize)]
pub struct AttemptResult {
    pub score: i32,
    pub total_points: i32,
}

pub async fn submit_attempt(
    State(state): State<AppState>,
    Path(quiz_id): Path<Uuid>,
    user: AuthUser,
    Json(payload): Json<SubmitAttemptRequest>,
) -> Result<Json<AttemptResult>, AppError> {
    #[derive(sqlx::FromRow)]
    struct QuestionPointsRow {
        id: Uuid,
        points: i32,
    }

    let questions = sqlx::query_as::<_, QuestionPointsRow>(
        "SELECT id, points FROM questions WHERE quiz_id = $1",
    )
    .bind(quiz_id)
    .fetch_all(&state.db)
    .await?;

    let total_points: i32 = questions.iter().map(|q| q.points).sum();
    let mut score = 0;

    for answer in &payload.answers {
        let is_correct: Option<bool> = sqlx::query_scalar(
            "SELECT is_correct FROM options WHERE id = $1 AND question_id = $2",
        )
        .bind(answer.option_id)
        .bind(answer.question_id)
        .fetch_optional(&state.db)
        .await?;

        if is_correct == Some(true) {
            if let Some(q) = questions.iter().find(|q| q.id == answer.question_id) {
                score += q.points;
            }
        }
    }

    sqlx::query(
        "INSERT INTO quiz_attempts (user_id, quiz_id, score, total_points) VALUES ($1, $2, $3, $4)",
    )
    .bind(user.user_id)
    .bind(quiz_id)
    .bind(score)
    .bind(total_points)
    .execute(&state.db)
    .await?;

    Ok(Json(AttemptResult { score, total_points }))
}

pub async fn like_quiz(
    State(state): State<AppState>,
    Path(quiz_id): Path<Uuid>,
    user: AuthUser,
) -> Result<StatusCode, AppError> {
    sqlx::query("INSERT INTO quiz_likes (user_id, quiz_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(user.user_id)
        .bind(quiz_id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn unlike_quiz(
    State(state): State<AppState>,
    Path(quiz_id): Path<Uuid>,
    user: AuthUser,
) -> Result<StatusCode, AppError> {
    sqlx::query("DELETE FROM quiz_likes WHERE user_id = $1 AND quiz_id = $2")
        .bind(user.user_id)
        .bind(quiz_id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
