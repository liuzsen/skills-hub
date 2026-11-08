use std::{fmt, future::Future};

use hub_domain::skill::{SkillFiles, SkillPath};

/// Storage namespace, not a replacement for the shared domain SkillId.
/// The caller supplies a fresh CSPRNG revision ID for every staging attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactRevision {
    skill_id: String,
    revision_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidArtifactRevision;

impl ArtifactRevision {
    pub fn new(
        skill_id: impl Into<String>,
        revision_id: impl Into<String>,
    ) -> Result<Self, InvalidArtifactRevision> {
        let skill_id = skill_id.into();
        let revision_id = revision_id.into();
        if !safe_segment(&skill_id) || !safe_segment(&revision_id) {
            return Err(InvalidArtifactRevision);
        }
        Ok(Self {
            skill_id,
            revision_id,
        })
    }

    pub fn skill_id(&self) -> &str {
        &self.skill_id
    }

    pub fn revision_id(&self) -> &str {
        &self.revision_id
    }
}

fn safe_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

/// Internal storage handle. Never accept this from a request or put it in an HTTP DTO.
/// No Display/serde implementation; Debug deliberately hides the key.
#[derive(Clone, PartialEq, Eq)]
pub struct ArtifactKey(String);

impl ArtifactKey {
    /// Used only by storage adapters, including reconstruction from trusted D1 rows.
    pub fn from_storage(value: String) -> Self {
        Self(value)
    }

    pub fn as_storage_key(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ArtifactKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ArtifactKey([redacted])")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactDescriptor {
    pub key: ArtifactKey,
    pub byte_len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileArtifact {
    pub path: SkillPath,
    pub artifact: ArtifactDescriptor,
    /// Valid UTF-8, not a MIME or security classification. Render as plain text.
    pub is_utf8: bool,
}

/// All artifacts are complete before this is returned. It does not mean D1 published them.
/// Persist this whole revision and change the current pointer in one guarded D1 operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedArtifacts {
    pub revision: ArtifactRevision,
    pub files: Vec<FileArtifact>,
    pub manifest: ArtifactDescriptor,
    pub zip: ArtifactDescriptor,
    pub total_bytes: u64,
}

/// Authorization and current-revision selection belong to the calling use case.
/// A missing object or incomplete upload is a technical failure, never a stale-revision fallback.
pub trait ArtifactStore: Send + Sync {
    /// Write an immutable revision from one validated file set. On error, retain all objects:
    /// a previous D1 commit might have succeeded even if its response was lost.
    fn stage(
        &self,
        revision: ArtifactRevision,
        files: SkillFiles,
    ) -> impl Future<Output = anyhow::Result<StagedArtifacts>> + Send;

    /// Bounded reads for authorized text/manifest access. Check length before buffering.
    /// ZIP downloads use the authorized descriptor at the HTTP/adapter streaming boundary.
    fn read(
        &self,
        artifact: &ArtifactDescriptor,
        max_bytes: u64,
    ) -> impl Future<Output = anyhow::Result<Vec<u8>>> + Send;
}
