use std::io::{Cursor, Write};

use anyhow::Context;
use hub_application::skills::artifacts::{ArtifactRevision, FileArtifact};
use hub_domain::skill::{MAX_FILES, MAX_PATH_BYTES, MAX_TOTAL_BYTES, SkillFiles};
use serde::Serialize;
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

// Includes local/central names, headers and room for library extra fields.
pub(super) const MAX_ZIP_BYTES: usize =
    MAX_TOTAL_BYTES + MAX_FILES * (2 * MAX_PATH_BYTES + 256) + 65536;

pub(super) fn stored_zip(files: &SkillFiles) -> anyhow::Result<Vec<u8>> {
    let output = Cursor::new(Vec::with_capacity(files.total_bytes()));
    let mut writer = ZipWriter::new(output);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(DateTime::default())
        .unix_permissions(0o644);
    for file in files.files() {
        writer
            .start_file(file.path().as_str(), options)
            .context("create ZIP entry")?;
        writer.write_all(file.bytes()).context("write ZIP entry")?;
    }
    let bytes = writer.finish().context("finish ZIP archive")?.into_inner();
    anyhow::ensure!(
        bytes.len() <= MAX_ZIP_BYTES,
        "generated ZIP exceeds artifact limit"
    );
    Ok(bytes)
}

#[derive(Serialize)]
struct Manifest<'a> {
    schema_version: u8,
    skill_id: &'a str,
    revision_id: &'a str,
    total_bytes: u64,
    files: Vec<ManifestFile<'a>>,
}

#[derive(Serialize)]
struct ManifestFile<'a> {
    path: &'a str,
    byte_len: u64,
    is_utf8: bool,
}

pub(super) fn manifest(
    revision: &ArtifactRevision,
    files: &[FileArtifact],
    total_bytes: u64,
) -> anyhow::Result<Vec<u8>> {
    let manifest = Manifest {
        schema_version: 1,
        skill_id: revision.skill_id(),
        revision_id: revision.revision_id(),
        total_bytes,
        files: files
            .iter()
            .map(|file| ManifestFile {
                path: file.path.as_str(),
                byte_len: file.artifact.byte_len,
                is_utf8: file.is_utf8,
            })
            .collect(),
    };
    serde_json::to_vec(&manifest).context("encode revision manifest")
}
