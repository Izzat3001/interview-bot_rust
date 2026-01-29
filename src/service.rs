use frankenstein::AsyncApi; 

// ВАЖНО: Импортируем асинхронный трейт, чтобы заработал .await
use frankenstein::AsyncTelegramApi; 

use frankenstein::{
    GetUpdatesParams, UpdateContent, SendMessageParams, 
    InlineKeyboardButton, InlineKeyboardMarkup, ReplyMarkup,
    MaybeInaccessibleMessage,
};
use crate::quiz::QuizBank;
use crate::session::UserSession; // Используем crate:: для доступа к соседним модулям
use crate::ui;
use std::collections::HashMap;

pub struct MyBotService {
    pub api: AsyncApi,
    pub bank: QuizBank,
}


impl MyBotService {
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let mut user_states: HashMap<i64, UserSession> = HashMap::new();
        let mut offset: i32 = 0; // Явно задаем тип Integer для Telegram

        println!("🚀 Бот запущен и слушает обновления...");

        // ВЕСЬ твой цикл loop переезжает сюда!
        loop {
            let params = GetUpdatesParams::builder()
                .offset(offset as i64) // Кастуем для API
                .timeout(10u32)
                .build();

            if let Ok(response) = self.api.get_updates(&params).await {
                for update in response.result {
                    offset = (update.update_id + 1) as i32;

                    match update.content {
                        UpdateContent::Message(message) => {
                            // Обработка обычных сообщений
                            let chat_id = message.chat.id;
                            let session = user_states.entry(chat_id).or_insert(UserSession::new());

                            if let Some(text) = message.text {
                                match text.as_str() {
                                    "/start" | "/help" => {
                                        ui::send_main_menu(&self.api, chat_id, ui::HELP_TEXT).await;
                                        // format!("{}", ui::HELP_TEXT);
                                    }
                                    "/stats" => {
                                        let (solved, total) =
                                            self.bank.get_user_stats(chat_id).await;
                                        let percent =
                                            if total > 0 { (solved * 100) / total } else { 0 };
                                        let response = format!(
                                            "📊 <b>Прогресс:</b>\n✅ {} из {}\n📈 {}%",
                                            solved, total, percent
                                        );
                                        let _ = self
                                            .api
                                            .send_message(
                                                &SendMessageParams::builder()
                                                    .chat_id(chat_id)
                                                    .text(response)
                                                    .parse_mode(frankenstein::ParseMode::Html)
                                                    .build(),
                                            )
                                            .await;
                                    }
                                    "/reset" => {
                                        // 1. Чистим базу
                                        self.bank.reset_user_progress(chat_id).await;
                                        // 2. Чистим локальную сессию (чтобы сбросить категорию)
                                        user_states.remove(&chat_id);

                                        let _ = self.api.send_message(&SendMessageParams::builder()
                                            .chat_id(chat_id)
                                            .text("🗑 <b>Прогресс сброшен!</b>\nНачинаем с чистого листа.")
                                            .parse_mode(frankenstein::ParseMode::Html)
                                            .build()).await;

                                        // Показываем меню снова
                                        ui::send_main_menu(&self.api, chat_id, "Выбери тему:")
                                            .await;
                                    }
                                    "📦 Basics" | "🦀 Ownership" | "🔄 Control Flow"
                                    | "📚 Collections" | "⚙️ Generics" | "🚀 Advanced" => {
                                        let category_str = text
                                            .split_whitespace()
                                            .skip(1)
                                            .collect::<Vec<_>>()
                                            .join(" ");

                                        // ИСПРАВЛЕНО: Теперь оба плеча if возвращают String (Ownership fix)
                                        let category = if category_str.is_empty() {
                                            text.clone()
                                        } else {
                                            category_str
                                        };

                                        session.set_category(&category);
                                        if let Some(q) =
                                            self.bank.get_question_smart(chat_id, &category).await
                                        {
                                            session.current_question_id = q.id;
                                            ui::send_question(&self.api, chat_id, &q).await;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        UpdateContent::CallbackQuery(query) => {
                            let _ = self
                                .api
                                .answer_callback_query(
                                    &frankenstein::AnswerCallbackQueryParams::builder()
                                        .callback_query_id(&query.id) // Передаем ID запроса
                                        .build(),
                                )
                                .await;

                            if let (Some(maybe_msg), Some(data)) = (query.message, query.data) {
                                // ИСПРАВЛЕНИЕ ЗДЕСЬ: Распаковываем MaybeInaccessibleMessage
                                if let MaybeInaccessibleMessage::Message(message) = maybe_msg {
                                    let chat_id = message.chat.id;
                                    // Твоя логика кнопок...
                                    if data.starts_with("next_") {
                                        if let Some(session) = user_states.get_mut(&chat_id) {
                                            // Пробуем получить следующий вопрос
                                            let next_q = self
                                                .bank
                                                .get_question_smart(
                                                    chat_id,
                                                    &session.current_category,
                                                )
                                                .await;

                                            match next_q {
                                                Some(new_q) => {
                                                    // Если вопрос нашелся — отправляем его
                                                    session.current_question_id = new_q.id;
                                                    ui::send_question(&self.api, chat_id, &new_q)
                                                        .await;
                                                }
                                                None => {
                                                    // БАГ ИСПРАВЛЕН: Если вопросов больше нет
                                                    // 1. Сбрасываем категорию в сессии
                                                    session.current_category = String::new();

                                                    // 2. Отправляем поздравление и меню
                                                    let finish_text = "🏆 <b>Поздравляю!</b>\nТы прошел все доступные вопросы в этой категории.";

                                                    let _ = self
                                                        .api
                                                        .send_message(
                                                            &SendMessageParams::builder()
                                                                .chat_id(chat_id)
                                                                .text(finish_text)
                                                                .parse_mode(
                                                                    frankenstein::ParseMode::Html,
                                                                )
                                                                .build(),
                                                        )
                                                        .await;

                                                    // 3. Показываем главное меню, чтобы юзер не потерялся
                                                    ui::send_main_menu(
                                                        &self.api,
                                                        chat_id,
                                                        "Выбери новую тему для изучения:",
                                                    )
                                                    .await;
                                                }
                                            }
                                        }
                                    } else if let Ok(choice) = data.parse::<usize>() {
                                        let session = user_states.get(&chat_id);
                                        let q_id =
                                            session.map(|s| s.current_question_id).unwrap_or(0);

                                        if let Some(q) = self.bank.get_question(q_id).await {
                                            self.bank.mark_as_answered(chat_id, q.id).await;

                                            let is_correct = q.is_correct(choice);
                                            let safe_explanation = ui::escape_html(&q.explanation);
                                            let safe_correct =
                                                ui::escape_html(&q.options[q.correct_option_id]);

                                            let response_text = if is_correct {
                                                format!(
                                                    "✅ <b>Верно!</b>\n\n💡 {}",
                                                    safe_explanation
                                                )
                                            } else {
                                                format!("❌ <b>Ошибка!</b>\nПраильный ответ: <code>{}</code>\n\n💡 {}", safe_correct, safe_explanation)
                                            };

                                            let next_btn = InlineKeyboardButton::builder()
                                                .text("🚀 Следующий")
                                                .callback_data(format!("next_{}", q.id))
                                                .build();

                                            let _ = self
                                                .api
                                                .send_message(
                                                    &SendMessageParams::builder()
                                                        .chat_id(chat_id)
                                                        .text(response_text)
                                                        .parse_mode(frankenstein::ParseMode::Html)
                                                        .reply_markup(
                                                            ReplyMarkup::InlineKeyboardMarkup(
                                                                InlineKeyboardMarkup::builder()
                                                                    .inline_keyboard(vec![vec![
                                                                        next_btn,
                                                                    ]])
                                                                    .build(),
                                                            ),
                                                        )
                                                        .build(),
                                                )
                                                .await;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
