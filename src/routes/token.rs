use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Form, Json};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::AppState;
use crate::grants::{Grant, random_token};
use crate::protocol::{GRANT_TYPE_AUTHORIZATION_CODE, TOKEN_TYPE_BEARER};

const BASIC_SCHEME_PREFIX: &str = "Basic ";

#[derive(Deserialize)]
pub struct TokenRequest {
    grant_type: String,
    code: String,
    redirect_uri: String,
    code_verifier: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Serialize)]
struct IdTokenClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    iat: u64,
    exp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<&'a str>,
    email: &'a str,
    email_verified: bool,
    name: &'a str,
}

enum TokenError {
    InvalidClient,
    InvalidGrant,
    UnsupportedGrantType,
    ServerError,
}

impl From<serde_json::Error> for TokenError {
    fn from(error: serde_json::Error) -> Self {
        eprintln!("oauth2-mock could not sign an ID token: {error}");
        Self::ServerError
    }
}

impl IntoResponse for TokenError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::InvalidClient => (StatusCode::UNAUTHORIZED, "invalid_client"),
            Self::InvalidGrant => (StatusCode::BAD_REQUEST, "invalid_grant"),
            Self::UnsupportedGrantType => (StatusCode::BAD_REQUEST, "unsupported_grant_type"),
            Self::ServerError => (StatusCode::INTERNAL_SERVER_ERROR, "server_error"),
        };
        (status, Json(json!({ "error": code }))).into_response()
    }
}

pub async fn exchange(
    State(app): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(request): Form<TokenRequest>,
) -> Response {
    match issue_tokens(&app, &headers, &request) {
        Ok(body) => Json(body).into_response(),
        Err(error) => error.into_response(),
    }
}

fn issue_tokens(
    app: &AppState,
    headers: &HeaderMap,
    request: &TokenRequest,
) -> Result<Value, TokenError> {
    if request.grant_type != GRANT_TYPE_AUTHORIZATION_CODE {
        return Err(TokenError::UnsupportedGrantType);
    }
    let client_id = authenticate(app, headers, request)?;
    let grant = app
        .grants
        .redeem(&request.code)
        .ok_or(TokenError::InvalidGrant)?;
    check_grant(&grant, &client_id, request)?;
    let id_token = sign_id_token(app, &grant)?;
    Ok(token_response(app, &id_token))
}

fn token_response(app: &AppState, id_token: &str) -> Value {
    json!({
        "access_token": random_token(),
        "token_type": TOKEN_TYPE_BEARER,
        "expires_in": app.config.token_lifetime.as_secs(),
        "id_token": id_token
    })
}

fn authenticate(
    app: &AppState,
    headers: &HeaderMap,
    request: &TokenRequest,
) -> Result<String, TokenError> {
    let credentials = if headers.contains_key(AUTHORIZATION) {
        basic_credentials(headers)
    } else {
        body_credentials(request)
    };
    let (client_id, client_secret) = credentials.ok_or(TokenError::InvalidClient)?;
    let is_known_client =
        client_id == app.config.client_id && client_secret == app.config.client_secret;

    if !is_known_client {
        return Err(TokenError::InvalidClient);
    }
    Ok(client_id)
}

fn body_credentials(request: &TokenRequest) -> Option<(String, String)> {
    Some((request.client_id.clone()?, request.client_secret.clone()?))
}

fn basic_credentials(headers: &HeaderMap) -> Option<(String, String)> {
    let header = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let encoded = header.strip_prefix(BASIC_SCHEME_PREFIX)?;
    let decoded = String::from_utf8(STANDARD.decode(encoded).ok()?).ok()?;
    let (client_id, client_secret) = decoded.split_once(':')?;
    Some((
        form_urlencoded_decoded(client_id)?,
        form_urlencoded_decoded(client_secret)?,
    ))
}

fn form_urlencoded_decoded(value: &str) -> Option<String> {
    let with_spaces = value.replace('+', " ");
    let decoded = percent_decode_str(&with_spaces).decode_utf8().ok()?;
    Some(decoded.into_owned())
}

fn check_grant(grant: &Grant, client_id: &str, request: &TokenRequest) -> Result<(), TokenError> {
    let is_same_client = grant.client_id == client_id;
    let is_same_redirect = grant.redirect_uri == request.redirect_uri;
    let is_valid_grant =
        is_same_client && is_same_redirect && grant.is_proven_by(&request.code_verifier);

    if !is_valid_grant {
        return Err(TokenError::InvalidGrant);
    }
    Ok(())
}

fn sign_id_token(app: &AppState, grant: &Grant) -> Result<String, TokenError> {
    let issued_at = seconds_since_epoch();
    let claims = IdTokenClaims {
        iss: &app.config.issuer,
        sub: &grant.user.id,
        aud: &grant.client_id,
        iat: issued_at,
        exp: issued_at + app.config.token_lifetime.as_secs(),
        nonce: grant.nonce.as_deref(),
        email: &grant.user.email,
        email_verified: true,
        name: &grant.user.name,
    };
    Ok(app.keys.sign(&claims)?)
}

fn seconds_since_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
