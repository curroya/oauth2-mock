use std::env::{self, VarError};
use std::fs;
use std::str::FromStr;
use std::time::Duration;

use url::Url;

use crate::users::UserEntry;

const DEFAULT_PORT: u16 = 5556;
const DEFAULT_ISSUER: &str = "http://localhost:5556";
const DEFAULT_CLIENT_ID: &str = "local-client";
const DEFAULT_CLIENT_SECRET: &str = "local-secret";
const DEFAULT_KEY_SEED: &str = "oauth2-mock";
const DEFAULT_TOKEN_LIFETIME_SECONDS: u64 = 3600;

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub issuer: String,
    pub base_path: String,
    pub browser_base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub key_seed: String,
    pub token_lifetime: Duration,
    pub users: Vec<UserEntry>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let issuer = env_or_default("ISSUER", DEFAULT_ISSUER);
        let token_lifetime_seconds =
            parsed_env_or_default("TOKEN_LIFETIME_SECONDS", DEFAULT_TOKEN_LIFETIME_SECONDS)?;

        Ok(Self {
            port: parsed_env_or_default("PORT", DEFAULT_PORT)?,
            base_path: issuer_path(&issuer)?,
            browser_base_url: env_or_default("BROWSER_BASE_URL", &issuer),
            issuer,
            client_id: env_or_default("CLIENT_ID", DEFAULT_CLIENT_ID),
            client_secret: env_or_default("CLIENT_SECRET", DEFAULT_CLIENT_SECRET),
            key_seed: env_or_default("KEY_SEED", DEFAULT_KEY_SEED),
            token_lifetime: Duration::from_secs(token_lifetime_seconds),
            users: configured_users()?,
        })
    }
}

pub fn issuer_path(issuer: &str) -> Result<String, String> {
    let url = Url::parse(issuer).map_err(|error| format!("ISSUER is not a URL: {error}"))?;
    Ok(url.path().trim_end_matches('/').to_owned())
}

fn configured_users() -> Result<Vec<UserEntry>, String> {
    let mut users = users_from_file()?;
    users.extend(users_from_csv());
    Ok(users)
}

fn users_from_file() -> Result<Vec<UserEntry>, String> {
    let Some(path) = optional_env("USERS_FILE")? else {
        return Ok(Vec::new());
    };
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("USERS_FILE {path} could not be read: {error}"))?;
    let users = parse_users(&path, &contents)?;
    reject_missing_emails(&path, &users)?;
    Ok(users)
}

fn parse_users(path: &str, contents: &str) -> Result<Vec<UserEntry>, String> {
    serde_json::from_str(contents)
        .map_err(|error| format!("USERS_FILE {path} is not a JSON list of users: {error}"))
}

fn reject_missing_emails(path: &str, users: &[UserEntry]) -> Result<(), String> {
    let has_user_without_email = users.iter().any(|user| user.email.trim().is_empty());
    if has_user_without_email {
        return Err(format!("USERS_FILE {path} has a user without an email"));
    }
    Ok(())
}

fn optional_env(name: &str) -> Result<Option<String>, String> {
    match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(format!("{name} is not valid UTF-8")),
    }
}

fn users_from_csv() -> Vec<UserEntry> {
    let emails = env::var("USERS").unwrap_or_default();
    emails
        .split(',')
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .map(UserEntry::for_email)
        .collect()
}

fn env_or_default(name: &str, fallback: &str) -> String {
    env::var(name).unwrap_or_else(|_| fallback.to_owned())
}

fn parsed_env_or_default<T: FromStr>(name: &str, fallback: T) -> Result<T, String> {
    let Ok(raw) = env::var(name) else {
        return Ok(fallback);
    };
    raw.parse()
        .map_err(|_| format!("{name} is not valid: {raw}"))
}
