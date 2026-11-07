pub mod authenticate_session;
pub mod issue_challenge;
pub mod logout;
pub mod verify_email;

use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const CHALLENGE_TTL: i64 = 600;
pub const RESEND_COOLDOWN: i64 = 60;
pub const MAX_ATTEMPTS: i64 = 5;
pub const SESSION_TTL: i64 = 30 * 24 * 60 * 60;

/// Never Debug/Serialize: this is server-only key material.
pub struct ChallengePepper(Vec<u8>);

impl ChallengePepper {
    pub fn from_hex(value: &str) -> anyhow::Result<Self> {
        let bytes =
            hex::decode(value).map_err(|_| anyhow::anyhow!("invalid AUTH_PEPPER encoding"))?;
        anyhow::ensure!(
            bytes.len() >= 32,
            "AUTH_PEPPER must contain at least 32 random bytes"
        );
        Ok(Self(bytes))
    }

    pub fn code_mac(&self, challenge_hash: &str, email: &str, code: &str) -> String {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.0).expect("HMAC accepts arbitrary key lengths");
        mac.update(b"skills-hub/email-challenge/v1\0");
        for value in [challenge_hash, email, code] {
            mac.update(value.as_bytes());
            mac.update(b"\0");
        }
        hex::encode(mac.finalize().into_bytes())
    }
}
