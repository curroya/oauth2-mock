use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::AppState;
use crate::protocol::{
    GRANT_TYPE_AUTHORIZATION_CODE, PKCE_METHOD, RESPONSE_TYPE_CODE, SIGNING_ALGORITHM,
};

pub async fn document(State(app): State<Arc<AppState>>) -> Json<Value> {
    let issuer = &app.config.issuer;
    let server_base = issuer.trim_end_matches('/');
    let browser_base = app.config.browser_base_url.trim_end_matches('/');
    Json(discovery_document(issuer, browser_base, server_base))
}

fn discovery_document(issuer: &str, browser_base: &str, server_base: &str) -> Value {
    json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{browser_base}/auth"),
        "token_endpoint": format!("{server_base}/token"),
        "jwks_uri": format!("{server_base}/keys"),
        "response_types_supported": [RESPONSE_TYPE_CODE],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": [SIGNING_ALGORITHM],
        "scopes_supported": ["openid", "email", "profile"],
        "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post"],
        "grant_types_supported": [GRANT_TYPE_AUTHORIZATION_CODE],
        "code_challenge_methods_supported": [PKCE_METHOD],
        "claims_supported": ["sub", "email", "email_verified", "name"]
    })
}

pub async fn keys(State(app): State<Arc<AppState>>) -> Json<Value> {
    Json(app.keys.jwks().clone())
}
