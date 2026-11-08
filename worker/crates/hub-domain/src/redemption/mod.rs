use crate::account::Actor;

pub const GUEST_TTL_SECONDS: i64 = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillOperation {
    Read,
    Download,
}

/// A server-derived digest, never the bearer token itself.
#[derive(Clone)]
pub struct GuestTokenHash(String);

impl GuestTokenHash {
    pub fn parse(value: String) -> Option<Self> {
        (value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        .then_some(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn has_global_access(actor: &Actor, operation: SkillOperation) -> bool {
    actor.account().is_some_and(|account| match operation {
        SkillOperation::Read => account.capabilities().can_view_all(),
        SkillOperation::Download => account.capabilities().can_download_all(),
    })
}
