use crate::api::{app::AppState, handlers, middleware as mw};

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::Method,
    middleware,
    routing::{delete, get, patch, post, put},
};
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::Span;

pub fn create_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::PATCH,
        ])
        .allow_origin(Any)
        .allow_headers(Any);

    let public = Router::new().route("/health", get(handlers::health_check));

    let protected = Router::new()
        .nest("/api/v1", api_routes(state.clone()))
        .layer(middleware::from_fn_with_state(state.clone(), mw::auth_jwt));

    Router::new()
        .merge(public)
        .merge(protected)
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &axum::extract::Request, _span: &Span| {
                    tracing::info!(
                        method = %request.method(),
                        uri = %request.uri(),
                        "→ request",
                    );
                })
                .on_response(
                    |response: &axum::response::Response,
                     latency: std::time::Duration,
                     _span: &Span| {
                        tracing::info!(
                            status = %response.status(),
                            latency_ms = latency.as_millis(),
                            "← response",
                        );
                    },
                ),
        )
        .layer(cors)
        .with_state(state)
}

fn api_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let transcription_routes = Router::new()
        .route("/transcriptions/stream", post(handlers::transcribe_stream))
        .layer(DefaultBodyLimit::max(60 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            mw::rate_limit_transcription,
        ));

    let translation_routes = Router::new()
        .route("/translations", post(handlers::translate))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            mw::rate_limit_translation,
        ));

    let note_routes = Router::new()
        .route("/notes/{input_hash}", get(handlers::get_note))
        .route("/notes/{input_hash}", put(handlers::upsert_note))
        .route("/notes/{input_hash}", delete(handlers::delete_note));

    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/transcriptions", get(handlers::all_transcriptions))
        .route("/transcriptions/{audio_signature}/rename", patch(handlers::rename_transcription))
        .route("/user/translations", get(handlers::user_translations))
        .merge(transcription_routes)
        .merge(translation_routes)
        .merge(note_routes)
}
