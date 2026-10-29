use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

use crate::users::User;

const TOKEN_BYTES: usize = 32;

pub struct Grant {
    pub client_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub nonce: Option<String>,
    pub user: User,
}

impl Grant {
    pub fn is_proven_by(&self, code_verifier: &str) -> bool {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
        challenge == self.code_challenge
    }
}

#[derive(Default)]
pub struct Grants {
    pending: Mutex<HashMap<String, Grant>>,
}

impl Grants {
    pub fn issue(&self, grant: Grant) -> String {
        let code = random_token();
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(code.clone(), grant);
        code
    }

    pub fn redeem(&self, code: &str) -> Option<Grant> {
        self.pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(code)
    }
}

pub fn random_token() -> String {
    let bytes: [u8; TOKEN_BYTES] = rand::random();
    URL_SAFE_NO_PAD.encode(bytes)
}
