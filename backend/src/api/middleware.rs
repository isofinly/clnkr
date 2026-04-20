use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation};

use crate::api::app::AppState;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct Claims {
    pub(super) sub: String,
    pub(super) exp: usize,
    pub(super) iat: usize,
}

/// Validates the Bearer JWT, rejects with 401 on failure, and inserts the
/// `user_id` string into request extensions so downstream handlers can
/// extract it with `Extension<String>`.
pub(super) async fn auth_jwt(
    State(state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = auth.token();

    let data = jsonwebtoken::decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(data.claims.sub);

    Ok(next.run(request).await)
}

struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

#[derive(Clone)]
pub(crate) struct RateLimiter {
    // requests per minute — kept for display / documentation purposes.
    _rpm: u32,
    /// tokens per second derived from rpm.
    refill_rate: f64,
    /// maximum burst == rpm (one full minute of tokens at once).
    capacity: f64,
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

impl RateLimiter {
    pub(crate) fn new(rpm: u32) -> Self {
        let capacity = rpm as f64;
        Self {
            _rpm: rpm,
            refill_rate: capacity / 60.0,
            capacity,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn check(&self, user_id: &str) -> bool {
        let mut map = self.buckets.lock().unwrap();
        let now = Instant::now();

        let bucket = map.entry(user_id.to_owned()).or_insert(Bucket {
            tokens: self.capacity,
            last_refill: now,
        });

        let elapsed_secs = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed_secs * self.refill_rate).min(self.capacity);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// 5 RPM — applied to the transcription route.
pub(super) async fn rate_limit_transcription(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    apply_rate_limit(&state.rate_limit_transcription, request, next).await
}

/// 15 RPM — applied to the translation route.
pub(super) async fn rate_limit_translation(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    apply_rate_limit(&state.rate_limit_translation, request, next).await
}

async fn apply_rate_limit(
    limiter: &RateLimiter,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // user_id was inserted by auth_jwt which must run before this middleware.
    let user_id = request
        .extensions()
        .get::<String>()
        .cloned()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if limiter.check(&user_id) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

/// Seconds until the bucket for `user_id` will have at least one token.
/// Returns 0 if a token is already available.
pub(super) fn seconds_until_ready(limiter: &RateLimiter, user_id: &str) -> u64 {
    let map = limiter.buckets.lock().unwrap();
    if let Some(bucket) = map.get(user_id) {
        let deficit = 1.0 - bucket.tokens;
        if deficit > 0.0 {
            return (deficit / limiter.refill_rate).ceil() as u64;
        }
    }
    0
}
