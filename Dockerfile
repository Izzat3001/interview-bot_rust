# --- ЭТАП 1: Сборка (Builder) ---
# Берем официальный образ Rust (версия slim весит меньше)
FROM rust:1.82-slim as builder

# Устанавливаем системные зависимости для сборки.
# pkg-config и libssl-dev ОБЯЗАТЕЛЬНЫ для сборки sqlx и работы с HTTPS
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Создаем рабочую папку внутри контейнера
WORKDIR /app

# Копируем все файлы из твоей папки в контейнер
COPY . .

# 1. Устанавливаем переменную окружения для компилятора
ENV SQLX_OFFLINE=true

# Собираем проект в режиме Release (максимальная скорость работы)
RUN cargo build --release

# --- ЭТАП 2: Запуск (Runtime) ---
# Берем легкую версию Debian (чтобы итоговый образ весил мало)
FROM debian:bookworm-slim

# Создаем рабочую папку
WORKDIR /app

# Устанавливаем сертификаты и OpenSSL.
# БЕЗ ca-certificates бот упадет с ошибкой при попытке связаться с Telegram!
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

# Копируем скомпилированный бинарник из первого этапа.
# ВНИМАНИЕ: Имя файла здесь совпадает с name в твоем Cargo.toml
COPY --from=builder /app/target/release/interview_bot_rust /app/interview_bot_rust

# Копируем файл с вопросами (он должен лежать рядом с бинарником)
COPY questions.json /app/questions.json

# Указываем команду запуска
CMD ["./interview_bot_rust"]
