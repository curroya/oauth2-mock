use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rsa::RsaPrivateKey;
use rsa::pkcs1v15::SigningKey;
use rsa::signature::{SignatureEncoding, Signer};
use rsa::traits::PublicKeyParts;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::protocol::SIGNING_ALGORITHM;

const KEY_BITS: usize = 2048;
const KEY_ID_LENGTH: usize = 16;

pub struct SigningKeys {
    signing_key: SigningKey<Sha256>,
    key_id: String,
    jwks: Value,
}

impl SigningKeys {
    pub fn from_seed(seed: &str) -> Result<Self, rsa::Error> {
        let mut seeded_random = StdRng::from_seed(Sha256::digest(seed.as_bytes()).into());
        let private_key = RsaPrivateKey::new(&mut seeded_random, KEY_BITS)?;
        let public_key = private_key.to_public_key();
        let modulus = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let exponent = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());
        let key_id = key_id_for(&modulus);
        let jwks = build_jwks(&key_id, &modulus, &exponent);

        Ok(Self {
            signing_key: SigningKey::new(private_key),
            key_id,
            jwks,
        })
    }

    pub fn jwks(&self) -> &Value {
        &self.jwks
    }

    pub fn sign<T: Serialize>(&self, claims: &T) -> Result<String, serde_json::Error> {
        let header = json!({ "alg": SIGNING_ALGORITHM, "typ": "JWT", "kid": self.key_id });
        let signing_input = format!("{}.{}", encode_json(&header)?, encode_json(claims)?);
        let signature = self.signing_key.sign(signing_input.as_bytes());
        Ok(format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        ))
    }
}

fn build_jwks(key_id: &str, modulus: &str, exponent: &str) -> Value {
    json!({
        "keys": [{
            "kty": "RSA",
            "use": "sig",
            "alg": SIGNING_ALGORITHM,
            "kid": key_id,
            "n": modulus,
            "e": exponent
        }]
    })
}

fn key_id_for(modulus: &str) -> String {
    let digest = URL_SAFE_NO_PAD.encode(Sha256::digest(modulus.as_bytes()));
    digest[..KEY_ID_LENGTH].to_owned()
}

fn encode_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    Ok(URL_SAFE_NO_PAD.encode(serde_json::to_vec(value)?))
}
