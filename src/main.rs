use frankenstein::AsyncApi;
use sqlx::postgres::PgPoolOptions;
use std::env;

mod quiz;
mod session;
mod ui;
mod service;

use crate::service::MyBotService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let conn_str = env::var("DATABASE_URL").expect("DATABASE_URL missing");
    let token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN missing");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&conn_str)
        .await?;

    let bank = quiz::QuizBank::with_pool(pool).await;
    bank.seed_database().await;

    let service = MyBotService {
        api: AsyncApi::new(&token),
        bank,
    };

    println!("🚀 Бот запущен!");
    service.run().await?; // Вызываем наш цикл

    Ok(())
}