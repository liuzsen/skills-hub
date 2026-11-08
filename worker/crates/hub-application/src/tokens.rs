use sha2::{Digest, Sha256};

pub trait TokenSource: Send + Sync {
    fn fill(&self, bytes: &mut [u8]) -> anyhow::Result<()>;
}

pub fn opaque(source: &impl TokenSource) -> anyhow::Result<String> {
    let mut bytes = [0; 32];
    source.fill(&mut bytes)?;
    Ok(hex::encode(bytes))
}

pub fn digest(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub fn is_opaque(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
