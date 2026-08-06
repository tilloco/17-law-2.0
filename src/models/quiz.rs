use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct QuizSummary {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub difficulty: String,
    pub like_count: i64,
    pub liked_by_me: bool,
}

#[derive(Debug, Serialize)]
pub struct QuizDetail {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub difficulty: String,
    pub questions: Vec<QuestionView>,
}

#[derive(Debug, Serialize)]
pub struct QuestionView {
    pub id: Uuid,
    pub text: String,
    pub options: Vec<OptionView>,
}

#[derive(Debug, Serialize)]
pub struct OptionView {
    pub id: Uuid,
    pub text: String,
}
