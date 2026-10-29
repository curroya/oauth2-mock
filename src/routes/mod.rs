mod authorize;
mod discovery;
mod sign_in_page;
mod token;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use crate::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/.well-known/openid-configuration",
            get(discovery::document),
        )
        .route("/keys", get(discovery::keys))
        .route("/auth", get(authorize::start).post(authorize::approve))
        .route("/token", post(token::exchange))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(state)
}
