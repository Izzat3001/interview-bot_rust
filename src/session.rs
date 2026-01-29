pub struct UserSession {
    pub current_question_id: i32,
    // Категория, которую выбрал пользователь (например, "Basics")
    pub current_category: String,
    // Текущая сложность, на которой он находится
    pub current_difficulty: i32,
}

impl UserSession {
    // Конструктор для создания новой сессии
    pub fn new() -> Self {
        Self {
            current_question_id: 0,
            current_category: "Basics".to_string(),
            current_difficulty: 1,
        }
    }

    // Метод для смены категории
    pub fn set_category(&mut self, category: &str) {
        self.current_category = category.to_string();
        // При смене категории всегда сбрасываем сложность на первую
        self.current_difficulty = 1; 
    }
}