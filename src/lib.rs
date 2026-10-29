mod config;
mod grants;
mod keys;
mod protocol;
mod routes;
mod users;

use std::sync::Arc;

use axum::Router;

pub use config::{Config, issuer_path};
use grants::Grants;
use keys::SigningKeys;
use users::Directory;
pub use users::{UserEntry, derived_id};

pub struct AppState {
    config: Config,
    keys: SigningKeys,
    grants: Grants,
    directory: Directory,
}

pub fn app(config: Config) -> Result<Router, rsa::Error> {
    let keys = SigningKeys::from_seed(&config.key_seed)?;
    let base_path = config.base_path.clone();
    let directory = Directory::new(&config.users);
    let state = Arc::new(AppState {
        config,
        keys,
        grants: Grants::default(),
        directory,
    });
    let routes = routes::router(state);

    if base_path.is_empty() {
        return Ok(routes);
    }
    Ok(Router::new().nest(&base_path, routes))
}
