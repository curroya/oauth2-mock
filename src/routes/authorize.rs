use std::sync::Arc;

use axum::Form;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;
use url::Url;

use crate::AppState;
use crate::grants::Grant;
use crate::protocol::{PKCE_METHOD, RESPONSE_TYPE_CODE};
use crate::routes::sign_in_page::render_sign_in_page;

#[derive(Deserialize)]
pub struct AuthorizationRequest {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    login_hint: Option<String>,
}

struct Approval {
    client_id: String,
    redirect_uri: String,
    callback: Url,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: String,
}

pub async fn start(
    State(app): State<Arc<AppState>>,
    Query(request): Query<AuthorizationRequest>,
) -> Response {
    respond(&app, &request, show_sign_in_page)
}

pub async fn approve(
    State(app): State<Arc<AppState>>,
    Form(request): Form<AuthorizationRequest>,
) -> Response {
    respond(&app, &request, |_, _| bad_request("login_hint is required"))
}

fn respond(
    app: &AppState,
    request: &AuthorizationRequest,
    without_email: fn(&AppState, &AuthorizationRequest) -> Response,
) -> Response {
    let approval = match approval_for(app, request) {
        Ok(approval) => approval,
        Err(rejection_reason) => return bad_request(rejection_reason),
    };
    let Some(email) = login_email(request) else {
        return without_email(app, request);
    };
    redirect_with_code(app, approval, email)
}

fn show_sign_in_page(app: &AppState, request: &AuthorizationRequest) -> Response {
    let forwarded_fields = forwarded_fields(request);
    Html(render_sign_in_page(
        &forwarded_fields,
        app.directory.users(),
    ))
    .into_response()
}

fn approval_for(app: &AppState, request: &AuthorizationRequest) -> Result<Approval, &'static str> {
    if request.client_id != app.config.client_id {
        return Err("unknown client_id");
    }
    if request.response_type != RESPONSE_TYPE_CODE {
        return Err("response_type must be code");
    }
    let code_challenge =
        s256_challenge(request).ok_or("a PKCE code_challenge with method S256 is required")?;
    let callback = Url::parse(&request.redirect_uri).map_err(|_| "redirect_uri is not a URL")?;

    Ok(Approval {
        client_id: request.client_id.clone(),
        redirect_uri: request.redirect_uri.clone(),
        callback,
        state: request.state.clone(),
        nonce: request.nonce.clone(),
        code_challenge,
    })
}

fn s256_challenge(request: &AuthorizationRequest) -> Option<String> {
    let is_s256 = request.code_challenge_method.as_deref() == Some(PKCE_METHOD);
    if !is_s256 {
        return None;
    }
    request.code_challenge.clone()
}

fn login_email(request: &AuthorizationRequest) -> Option<String> {
    let email = request.login_hint.as_deref()?.trim();
    if email.is_empty() {
        return None;
    }
    Some(email.to_owned())
}

fn bad_request(reason: &'static str) -> Response {
    (StatusCode::BAD_REQUEST, reason).into_response()
}

fn redirect_with_code(app: &AppState, approval: Approval, email: String) -> Response {
    let code = app.grants.issue(Grant {
        client_id: approval.client_id,
        redirect_uri: approval.redirect_uri,
        code_challenge: approval.code_challenge,
        nonce: approval.nonce,
        user: app.directory.user_for(&email),
    });
    let callback = callback_with(approval.callback, &code, approval.state.as_deref());
    Redirect::to(callback.as_str()).into_response()
}

fn callback_with(mut callback: Url, code: &str, state: Option<&str>) -> Url {
    callback.query_pairs_mut().append_pair("code", code);
    if let Some(state) = state {
        callback.query_pairs_mut().append_pair("state", state);
    }
    callback
}

fn forwarded_fields(request: &AuthorizationRequest) -> Vec<(&'static str, &str)> {
    let required_fields = [
        ("response_type", request.response_type.as_str()),
        ("client_id", request.client_id.as_str()),
        ("redirect_uri", request.redirect_uri.as_str()),
    ];
    let optional_fields = [
        ("state", &request.state),
        ("nonce", &request.nonce),
        ("code_challenge", &request.code_challenge),
        ("code_challenge_method", &request.code_challenge_method),
    ];
    let present_optional_fields = optional_fields
        .into_iter()
        .filter_map(|(name, value)| Some((name, value.as_deref()?)));

    required_fields
        .into_iter()
        .chain(present_optional_fields)
        .collect()
}
