use sqlx::postgres::PgPool;

use crate::{api::middleware::RateLimiter, llm::provider::UnifiedModelClient};

pub struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) llm: UnifiedModelClient,
    pub(crate) jwt_secret: String,
    /// 5 RPM per key — guarding the transcription SSE endpoint.
    pub(crate) rate_limit_transcription: RateLimiter,
    /// 15 RPM per key — guarding the translation endpoint.
    pub(crate) rate_limit_translation: RateLimiter,
}

impl AppState {
    pub fn new(pool: PgPool, llm: UnifiedModelClient, jwt_secret: String) -> Self {
        Self {
            pool,
            llm,
            jwt_secret,
            rate_limit_transcription: RateLimiter::new(5),
            rate_limit_translation: RateLimiter::new(15),
        }
    }
}
