CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    google_id VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    avatar_url TEXT,
    role VARCHAR(20) NOT NULL DEFAULT 'user',                -- 'user' | 'admin'
    subscription_tier VARCHAR(20) NOT NULL DEFAULT 'free',   -- 'free' | 'premium'
    subscription_expires_at TIMESTAMPTZ,
    preferred_language VARCHAR(5) NOT NULL DEFAULT 'uz',     -- 'uz' | 'ru' | 'en'
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE quizzes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category VARCHAR(100) NOT NULL,
    difficulty VARCHAR(20) NOT NULL DEFAULT 'medium',
    is_published BOOLEAN NOT NULL DEFAULT false,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE quiz_translations (
    quiz_id UUID NOT NULL REFERENCES quizzes(id) ON DELETE CASCADE,
    language_code VARCHAR(5) NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    PRIMARY KEY (quiz_id, language_code)
);

CREATE TABLE questions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    quiz_id UUID NOT NULL REFERENCES quizzes(id) ON DELETE CASCADE,
    order_index INT NOT NULL,
    points INT NOT NULL DEFAULT 1
);

CREATE TABLE question_translations (
    question_id UUID NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    language_code VARCHAR(5) NOT NULL,
    question_text TEXT NOT NULL,
    PRIMARY KEY (question_id, language_code)
);

CREATE TABLE options (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    question_id UUID NOT NULL REFERENCES questions(id) ON DELETE CASCADE,
    order_index INT NOT NULL,
    is_correct BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE option_translations (
    option_id UUID NOT NULL REFERENCES options(id) ON DELETE CASCADE,
    language_code VARCHAR(5) NOT NULL,
    option_text TEXT NOT NULL,
    PRIMARY KEY (option_id, language_code)
);

CREATE TABLE quiz_likes (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    quiz_id UUID NOT NULL REFERENCES quizzes(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, quiz_id)
);

CREATE TABLE quiz_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    quiz_id UUID NOT NULL REFERENCES quizzes(id) ON DELETE CASCADE,
    score INT NOT NULL,
    total_points INT NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_quiz_attempts_user ON quiz_attempts(user_id);
CREATE INDEX idx_quiz_translations_lang ON quiz_translations(language_code);
CREATE INDEX idx_questions_quiz ON questions(quiz_id);
CREATE INDEX idx_options_question ON options(question_id);
