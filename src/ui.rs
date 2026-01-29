// ВАЖНО: Импортируем асинхронный трейт, чтобы заработал .await
use frankenstein::AsyncTelegramApi; 

use frankenstein::{
    InlineKeyboardButton, InlineKeyboardMarkup, ReplyMarkup, 
    SendMessageParams, ParseMode,
};
use crate::quiz::QuizQuestion;

pub async fn send_question<T>(api: &T, chat_id: i64, question: &QuizQuestion)
where 
    T: AsyncTelegramApi + Sync,
    T::Error: std::fmt::Debug,
{
    // Вспомогательная функция для экранирования HTML-символов
    fn escape_html(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    // 1. Генерируем звезды сложности
    let stars = "⭐".repeat(question.difficulty as usize);

    // 2. Начало текста сообщения (Категорию и звезды можно не экранировать, если там нет < >)
    let mut question_text = format!(
        "<b>Категория:</b> {}\n\
         <b>Сложность:</b> {}\n\n\
         {}",
        escape_html(&question.category),
        stars,
        escape_html(&question.text) // ЭКРАНИРУЕМ ТЕКСТ ВОПРОСА
    );

    // 3. Формируем список вариантов и кнопки
    let mut buttons_row = Vec::new();

    for (i, option) in question.options.iter().enumerate() {
        // А. Добавляем экранированный текст ответа В СООБЩЕНИЕ
        // Оборачиваем код в <code>, чтобы он выглядел профессионально
        let safe_option = escape_html(option);
        question_text.push_str(&format!("\n\n{}. <code>{}</code>", i + 1, safe_option));

        // Б. Создаем кнопку (тут просто цифры, они безопасны)
        let btn_text = (i + 1).to_string();
        let callback_data = i.to_string();

        buttons_row.push(
            InlineKeyboardButton::builder()
                .text(btn_text)
                .callback_data(callback_data)
                .build(),
        );
    }

    // 4. Собираем клавиатуру
    let keyboard = InlineKeyboardMarkup::builder()
        .inline_keyboard(vec![buttons_row])
        .build();

    let params = SendMessageParams::builder()
        .chat_id(chat_id)
        .text(question_text)
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::InlineKeyboardMarkup(keyboard))
        .build();

    // Отправляем
    if let Err(e) = api.send_message(&params).await {
        eprintln!("🚨 Ошибка при отправке вопроса: {:?}", e); 
    }
}

// Помнишь, мы хотели вынести меню в ui?
pub async fn send_main_menu<T>(api: &T, chat_id: i64, text: &str)
where 
    T: AsyncTelegramApi + Sync,
    T::Error: std::fmt::Debug,
{
    use frankenstein::{KeyboardButton, ReplyKeyboardMarkup};

    // Строим сетку как на фото
    let buttons = vec![
        // Первая строка: 2 кнопки
        vec![
            KeyboardButton::builder().text("📦 Basics").build(),
            KeyboardButton::builder().text("🦀 Ownership").build(),
        ],
        // Вторая строка: 2 кнопки
        vec![
            KeyboardButton::builder().text("📚 Collections").build(),
            KeyboardButton::builder().text("🔄 Control Flow").build(),
        ],
        // Третья строка: 1 кнопка (Generics)
        vec![KeyboardButton::builder().text("⚙️ Generics").build()],
        // Четвертая строка: 1 кнопка (Advanced)
        vec![KeyboardButton::builder().text("🚀 Advanced").build()],
    ];

    let keyboard = ReplyKeyboardMarkup::builder()
        .keyboard(buttons)
        .resize_keyboard(true) // Чтобы кнопки не были на пол-экрана
        .build();

    let params = SendMessageParams::builder()
        .chat_id(chat_id)
        .text(text)
        .reply_markup(ReplyMarkup::ReplyKeyboardMarkup(keyboard))
        .build();

    let _ = api.send_message(&params).await;
}

pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// Наша константа для DRY (Don't Repeat Yourself)
pub const HELP_TEXT: &str = "<b>🦀 Rust Interview Bot — Твой наставник</b>\n\n\
    Я помогу тебе подготовиться к собеседованию по Rust. \n\
    Вот что я умею:\n\n\
    /start — Открыть главное меню и выбрать тему\n\
    /stats — Посмотреть, сколько вопросов ты уже затащил\n\
    /reset — Сбросить прогресс и начать заново\n\
    /help  — Показать это сообщение\n\n\
    <i>Совет: Читай объяснения после ответов, там самая соль!</i>";
