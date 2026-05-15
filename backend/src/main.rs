use anyhow::Context;

use sqlx;
use transcriber_api::{api::app::AppState, *};

use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let trancription_api_key =
        env::var("GEMINI_TRANSCRIBE_API_KEY").expect("GEMINI_TRANSCRIBE_API_KEY must be set");
    let translation_api_key =
        env::var("GEMINI_TRANSLATE_API_KEY").expect("GEMINI_TRANSLATE_API_KEY must be set");
    let openrouter_api_key: Option<String> = env::var("OPENROUTER_API_KEY").ok();

    let psql_connection_string = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    logger::init_logger_with_config(logger::LoggerConfig {
        level: "debug".to_string(),
        file_path: None,
        file_level: "info".to_string(),
    });

    let pool = sqlx::PgPool::connect(&psql_connection_string)
        .await
        .context("failed to establish initial connection to psql")
        .unwrap();

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run migrations")
        .unwrap();

    let llm = llm::provider::UnifiedModelClient::new(
        (trancription_api_key, translation_api_key),
        openrouter_api_key,
    );

    let state = AppState::new(pool, llm, jwt_secret);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();

    let router = transcriber_api::api::router::create_router(std::sync::Arc::new(state));

    axum::serve(listener, router).await?;
    Ok(())
}
