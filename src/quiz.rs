use sqlx::PgPool; // Теперь используем Postgres
use sqlx::Row;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Clone, Debug)]
pub struct QuizQuestion {
    #[serde(default)]
    pub id: i32, // В Postgres для SERIAL лучше i32
    pub text: String,
    pub options: Vec<String>,
    pub correct_option_id: usize,
    pub explanation: String,
    pub difficulty: i32,
    #[serde(default)]
    pub category: String,
}

impl QuizQuestion {
    pub fn is_correct(&self, index: usize) -> bool {
        index == self.correct_option_id
    }
}

pub struct QuizBank {
    db_pool: PgPool,
}

#[derive(Debug, serde::Deserialize)]
pub struct QuestionJson {
    pub category: String,
    pub difficulty: i32,
    pub text: String,
    pub options: Vec<String>,
    pub correct_option_id: i32,
    pub explanation: String,
}

impl QuizBank {
    // Конструктор для Shuttle (принимает уже готовый пул)
    pub async fn with_pool(pool: PgPool) -> Self {
        // Создаем таблицы, если их нет
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS questions (
                id SERIAL PRIMARY KEY,
                text TEXT NOT NULL UNIQUE, 
                options TEXT NOT NULL,
                correct_id INTEGER NOT NULL,
                explanation TEXT NOT NULL,
                difficulty INTEGER DEFAULT 1,
                category TEXT NOT NULL DEFAULT 'Basics'
            )",
        )
        .execute(&pool)
        .await
        .expect("🚨 Не удалось создать таблицу questions");

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_answers (
                chat_id BIGINT NOT NULL,
                question_id INTEGER NOT NULL,
                PRIMARY KEY (chat_id, question_id)
            )",
        )
        .execute(&pool)
        .await
        .expect("🚨 Не удалось создать таблицу user_answers");

        QuizBank { db_pool: pool }
    }

    pub async fn seed_database(&self) {

        let file_content = match fs::read_to_string("questions.json") {
            Ok(c) => c,
            Err(e) => {
                eprintln!("⚠️ Файл questions.json не найден: {}", e);
                return;
            }
        };

        let questions: Vec<QuestionJson> = match serde_json::from_str(&file_content) {
            Ok(q) => q,
            Err(e) => {
                eprintln!("🚨 Ошибка JSON: {}", e);
                return;
            }
        };

        let mut added_count = 0;

        for q in questions {
            let options_str = q.options.join("|");

            // В Postgres 'INSERT OR IGNORE' заменяется на 'ON CONFLICT DO NOTHING'
            let result = sqlx::query(
                "INSERT INTO questions (category, difficulty, text, options, correct_id, explanation) 
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (text) DO NOTHING"
            )
            .bind(q.category)
            .bind(q.difficulty)
            .bind(q.text)
            .bind(options_str)
            .bind(q.correct_option_id)
            .bind(q.explanation)
            .execute(&self.db_pool)
            .await;

            if let Ok(res) = result {
                if res.rows_affected() > 0 {
                    added_count += 1;
                }
            }
        }

        if added_count > 0 {
            println!("📥 Добавлено новых вопросов из файла: {}", added_count);
        } else {
            println!("✅ База актуальна, новых вопросов не найдено.");
        }
    }

    pub async fn get_question(&self, id: i32) -> Option<QuizQuestion> {
        let row = sqlx::query(
            "SELECT id, text, options, correct_id, explanation, difficulty, category FROM questions WHERE id = $1",
        )
        .bind(id) 
        .fetch_optional(&self.db_pool)
        .await
        .ok()?; 

        if let Some(r) = row {
            let options_raw: String = r.get("options");
            Some(QuizQuestion {
                id: r.get("id"),
                text: r.get("text"),
                options: options_raw.split('|').map(|s| s.to_string()).collect(),
                correct_option_id: r.get::<i32, _>("correct_id") as usize,
                explanation: r.get("explanation"),
                difficulty: r.get("difficulty"),
                category: r.get("category"),
            })
        } else {
            None
        }
    }

    pub async fn get_question_smart(&self, chat_id: i64, category: &str) -> Option<QuizQuestion> {
        // RANDOM() в Postgres работает так же, как в SQLite
        let row = sqlx::query(
            "SELECT id, text, options, correct_id, explanation, difficulty, category 
             FROM questions 
             WHERE category = $1 
             AND id NOT IN (SELECT question_id FROM user_answers WHERE chat_id = $2) 
             ORDER BY RANDOM() 
             LIMIT 1",
        )
        .bind(category) 
        .bind(chat_id) 
        .fetch_optional(&self.db_pool)
        .await
        .ok()?;

        if let Some(r) = row {
            return Some(QuizQuestion {
                id: r.get("id"),
                text: r.get("text"),
                options: r
                    .get::<String, _>("options")
                    .split('|')
                    .map(|s| s.to_string())
                    .collect(),
                correct_option_id: r.get::<i32, _>("correct_id") as usize,
                explanation: r.get("explanation"),
                difficulty: r.get("difficulty"),
                category: r.get("category"),
            });
        }
        None 
    }

    pub async fn mark_as_answered(&self, chat_id: i64, question_id: i32) {
        sqlx::query("INSERT INTO user_answers (chat_id, question_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(chat_id)
            .bind(question_id)
            .execute(&self.db_pool)
            .await
            .ok();
    }

    pub async fn get_user_stats(&self, chat_id: i64) -> (i64, i64) {
        // В Postgres COUNT возвращает i64
        let solved: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT question_id) FROM user_answers WHERE chat_id = $1",
        )
        .bind(chat_id)
        .fetch_one(&self.db_pool)
        .await
        .unwrap_or(0);

        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM questions")
            .fetch_one(&self.db_pool)
            .await
            .unwrap_or(0);

        (solved, total)
    }

    pub async fn reset_user_progress(&self, user_id: i64) {
    // 1. Сначала просто выполняем запрос и ждем результат
    let result = sqlx::query("DELETE FROM user_answers WHERE chat_id = $1")
        .bind(user_id)
        .execute(&self.db_pool)
        .await;

    // 2. Обрабатываем результат по-человечески
    match result {
        Ok(_) => println!("✅ Прогресс пользователя {} успешно сброшен", user_id),
        Err(e) => {
            // Если ты видишь это сообщение, значит таблицы все еще НЕТ
            eprintln!("🚨 Ошибка сброса прогресса для {}: {}", user_id, e);
        }
    }
}
}