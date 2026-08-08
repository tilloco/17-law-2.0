use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::extractors::AuthUser;
use crate::error::AppError;
use crate::AppState;

#[derive(Serialize)]
pub struct MyStats {
    pub total_attempts: i64,
    pub quizzes_completed: i64,
    pub average_score_pct: f64,
    pub best_score_pct: f64,
    pub attempts_this_week: i64,
    pub liked_count: i64,
}

pub async fn my_stats(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<MyStats>, AppError> {
    let row = sqlx::query_as::<_, (i64, i64, Option<f64>, Option<f64>, i64, i64)>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM quiz_attempts WHERE user_id = $1) AS total_attempts,
            (SELECT COUNT(DISTINCT quiz_id) FROM quiz_attempts WHERE user_id = $1) AS quizzes_completed,
            (SELECT AVG(score::float8 / NULLIF(total_points, 0) * 100)
                FROM quiz_attempts WHERE user_id = $1) AS average_score_pct,
            (SELECT MAX(score::float8 / NULLIF(total_points, 0) * 100)
                FROM quiz_attempts WHERE user_id = $1) AS best_score_pct,
            (SELECT COUNT(*) FROM quiz_attempts
                WHERE user_id = $1 AND completed_at >= now() - interval '7 days') AS attempts_this_week,
            (SELECT COUNT(*) FROM quiz_likes WHERE user_id = $1) AS liked_count
        "#,
    )
    .bind(user.user_id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(MyStats {
        total_attempts: row.0,
        quizzes_completed: row.1,
        average_score_pct: row.2.unwrap_or(0.0),
        best_score_pct: row.3.unwrap_or(0.0),
        attempts_this_week: row.4,
        liked_count: row.5,
    }))
}

#[derive(Deserialize)]
pub struct LangAndLimit {
    #[serde(default = "default_lang")]
    pub lang: String,
    pub limit: Option<i64>,
}

fn default_lang() -> String {
    "uz".to_string()
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AttemptHistoryRow {
    pub attempt_id: Uuid,
    pub quiz_id: Uuid,
    pub quiz_title: String,
    pub category: String,
    pub score: i32,
    pub total_points: i32,
    pub completed_at: DateTime<Utc>,
}

pub async fn my_attempts(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<LangAndLimit>,
) -> Result<Json<Vec<AttemptHistoryRow>>, AppError> {
    let limit = params.limit.unwrap_or(20).min(100);

    let rows = sqlx::query_as::<_, AttemptHistoryRow>(
        r#"
        SELECT
            qa.id AS attempt_id,
            qa.quiz_id,
            COALESCE(qt.title, '(untitled)') AS quiz_title,
            q.category,
            qa.score,
            qa.total_points,
            qa.completed_at
        FROM quiz_attempts qa
        JOIN quizzes q ON q.id = qa.quiz_id
        LEFT JOIN quiz_translations qt ON qt.quiz_id = q.id AND qt.language_code = $3
        WHERE qa.user_id = $1
        ORDER BY qa.completed_at DESC
        LIMIT $2
        "#,
    )
    .bind(user.user_id)
    .bind(limit)
    .bind(&params.lang)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(rows))
}