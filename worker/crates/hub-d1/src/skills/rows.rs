use std::collections::BTreeSet;

use hub_application::{
    skills::{
        RevisionSnapshot, SkillSummary,
        artifacts::{
            ArtifactDescriptor, ArtifactKey, ArtifactRevision, FileArtifact, StagedArtifacts,
        },
    },
    tokens,
};
use hub_domain::skill::{
    MAX_FILE_BYTES, MAX_FILES, MAX_TOTAL_BYTES, SkillId, SkillMetadata, SkillPath,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub(super) struct SummaryRow {
    pub skill_id: String,
    pub name: String,
    pub description: String,
    pub tags_json: String,
    pub updated_at: i64,
}

impl SummaryRow {
    pub fn into_summary(self) -> anyhow::Result<SkillSummary> {
        let id = SkillId::parse(self.skill_id)
            .map_err(|_| anyhow::anyhow!("invalid stored skill id"))?;
        let tags = serde_json::from_str(&self.tags_json)
            .map_err(|_| anyhow::anyhow!("invalid stored skill tags"))?;
        let metadata = SkillMetadata::new(self.name, self.description, tags)
            .map_err(|_| anyhow::anyhow!("invalid stored skill summary"))?;
        anyhow::ensure!(self.updated_at >= 0, "invalid stored skill timestamp");
        Ok(SkillSummary {
            id,
            metadata,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct FileRow {
    path: String,
    key: String,
    byte_len: u64,
    is_utf8: bool,
}

pub(super) fn encode_files(artifacts: &StagedArtifacts) -> anyhow::Result<String> {
    let rows: Vec<_> = artifacts
        .files
        .iter()
        .map(|file| FileRow {
            path: file.path.as_str().to_owned(),
            key: file.artifact.key.as_storage_key().to_owned(),
            byte_len: file.artifact.byte_len,
            is_utf8: file.is_utf8,
        })
        .collect();
    serde_json::to_string(&rows)
        .map_err(|_| anyhow::anyhow!("encode publication file manifest failed"))
}

#[derive(Deserialize)]
pub(super) struct SnapshotRow {
    skill_id: String,
    revision_id: String,
    name: String,
    description: String,
    tags_json: String,
    updated_at: i64,
    files_json: String,
    manifest_key: String,
    manifest_size: u64,
    zip_key: String,
    zip_size: u64,
    total_bytes: u64,
}

impl SnapshotRow {
    pub fn into_snapshot(self) -> anyhow::Result<RevisionSnapshot> {
        anyhow::ensure!(
            tokens::is_opaque(&self.revision_id),
            "invalid stored revision id"
        );
        let revision = ArtifactRevision::new(self.skill_id.clone(), self.revision_id.clone())
            .map_err(|_| anyhow::anyhow!("invalid stored artifact namespace"))?;
        let prefix = format!("skills/{}/{}", self.skill_id, self.revision_id);
        anyhow::ensure!(
            self.manifest_key == format!("{prefix}/manifest.json")
                && self.zip_key == format!("{prefix}/download.zip"),
            "invalid stored revision artifact location"
        );
        anyhow::ensure!(
            self.manifest_size <= 2 * 1024 * 1024
                && self.zip_size <= 9 * 1024 * 1024
                && self.files_json.len() <= 2 * 1024 * 1024,
            "stored artifact limit exceeded"
        );
        let rows: Vec<FileRow> = serde_json::from_str(&self.files_json)
            .map_err(|_| anyhow::anyhow!("invalid stored file manifest"))?;
        anyhow::ensure!(
            !rows.is_empty() && rows.len() <= MAX_FILES,
            "invalid stored file count"
        );
        let mut files = Vec::with_capacity(rows.len());
        let mut total_bytes = 0u64;
        let mut names = BTreeSet::new();
        for (index, row) in rows.into_iter().enumerate() {
            let path = SkillPath::parse(row.path)
                .map_err(|_| anyhow::anyhow!("invalid stored file path"))?;
            anyhow::ensure!(
                row.key == format!("{prefix}/files/{index:03}")
                    && row.byte_len <= MAX_FILE_BYTES as u64,
                "invalid stored file artifact"
            );
            total_bytes = total_bytes
                .checked_add(row.byte_len)
                .ok_or_else(|| anyhow::anyhow!("stored file total overflow"))?;
            anyhow::ensure!(
                names.insert(path.as_str().to_owned()),
                "duplicate stored file path"
            );
            files.push(FileArtifact {
                path,
                artifact: ArtifactDescriptor {
                    key: ArtifactKey::from_storage(row.key),
                    byte_len: row.byte_len,
                },
                is_utf8: row.is_utf8,
            });
        }
        for file in &files {
            for (index, _) in file.path.as_str().match_indices('/') {
                anyhow::ensure!(
                    !names.contains(&file.path.as_str()[..index]),
                    "stored file directory conflict"
                );
            }
        }
        anyhow::ensure!(
            files.windows(2).all(|pair| pair[0].path < pair[1].path),
            "stored files are not sorted"
        );
        anyhow::ensure!(
            total_bytes == self.total_bytes && total_bytes <= MAX_TOTAL_BYTES as u64,
            "invalid stored total size"
        );
        let summary = SummaryRow {
            skill_id: self.skill_id,
            name: self.name,
            description: self.description,
            tags_json: self.tags_json,
            updated_at: self.updated_at,
        }
        .into_summary()?;
        Ok(RevisionSnapshot {
            summary,
            artifacts: StagedArtifacts {
                revision,
                files,
                total_bytes,
                manifest: ArtifactDescriptor {
                    key: ArtifactKey::from_storage(self.manifest_key),
                    byte_len: self.manifest_size,
                },
                zip: ArtifactDescriptor {
                    key: ArtifactKey::from_storage(self.zip_key),
                    byte_len: self.zip_size,
                },
            },
        })
    }
}
