use sqlx::postgres::PgPool;

use crate::{api::middleware::RateLimiter, llm::gemini::GeminiClient};

pub struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) gemini: GeminiClient,
    pub(crate) jwt_secret: String,
    /// 5 RPM — guarding the transcription SSE endpoint.
    pub(crate) rate_limit_transcription: RateLimiter,
    /// 15 RPM — guarding the translation endpoint.
    pub(crate) rate_limit_translation: RateLimiter,
}

impl AppState {
    pub fn new(pool: PgPool, gemini: GeminiClient, jwt_secret: String) -> Self {
        Self {
            pool,
            gemini,
            jwt_secret,
            rate_limit_transcription: RateLimiter::new(5),
            rate_limit_translation: RateLimiter::new(15),
        }
    }
}
