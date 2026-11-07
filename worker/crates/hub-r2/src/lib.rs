mod encoding;

use std::future::Future;

use anyhow::{anyhow, ensure};
use hub_application::skills::artifacts::{
    ArtifactDescriptor, ArtifactKey, ArtifactRevision, ArtifactStore, FileArtifact, StagedArtifacts,
};
use hub_domain::skill::SkillFiles;
use worker::{
    Bucket, Conditional, HttpMetadata, ResponseBody,
    send::{SendFuture, SendWrapper},
};

/// Request-scoped binding. JS values and futures are bridged only inside this adapter.
/// Construct/use only in the single-threaded Workers runtime, not native thread pools.
pub struct R2ArtifactStore {
    bucket: SendWrapper<Bucket>,
}

impl R2ArtifactStore {
    pub fn new(bucket: Bucket) -> Self {
        Self {
            bucket: SendWrapper::new(bucket),
        }
    }

    /// Called only after the application authorizes this exact revision descriptor.
    /// Pass the body directly to Response::from_body; do not buffer a ZIP in Wasm.
    /// This SDK-specific handoff stays outside the application port.
    pub async fn open_body(&self, artifact: &ArtifactDescriptor) -> anyhow::Result<ResponseBody> {
        let object = self
            .bucket
            .get(artifact.key.as_storage_key())
            .execute()
            .await
            .map_err(|error| sdk_failure("open artifact", error))?
            .ok_or_else(|| anyhow!("published artifact is missing"))?;
        ensure!(
            object.size() == artifact.byte_len,
            "published artifact size mismatch"
        );
        let body = object
            .body()
            .ok_or_else(|| anyhow!("published artifact body is missing"))?;
        body.response_body()
            .map_err(|error| sdk_failure("open artifact body", error))
    }
}

impl ArtifactStore for R2ArtifactStore {
    fn stage(
        &self,
        revision: ArtifactRevision,
        files: SkillFiles,
    ) -> impl Future<Output = anyhow::Result<StagedArtifacts>> + Send {
        SendFuture::new(async move {
            let total_bytes = files.total_bytes() as u64;
            let prefix = format!("skills/{}/{}", revision.skill_id(), revision.revision_id());
            let file_artifacts: Vec<_> = files
                .files()
                .iter()
                .enumerate()
                .map(|(index, file)| FileArtifact {
                    path: file.path().clone(),
                    artifact: descriptor(format!("{prefix}/files/{index:03}"), file.bytes().len()),
                    is_utf8: file.is_utf8(),
                })
                .collect();
            // Encode both from this single validated set before consuming any file bytes.
            let zip_bytes = encoding::stored_zip(&files)?;
            let manifest_bytes = encoding::manifest(&revision, &file_artifacts, total_bytes)?;
            let zip = descriptor(format!("{prefix}/download.zip"), zip_bytes.len());
            let manifest = descriptor(format!("{prefix}/manifest.json"), manifest_bytes.len());

            for (file, artifact) in files.into_files().into_iter().zip(&file_artifacts) {
                put_immutable(
                    &self.bucket,
                    &artifact.artifact,
                    file.into_bytes(),
                    "application/octet-stream",
                )
                .await?;
            }
            put_immutable(&self.bucket, &zip, zip_bytes, "application/zip").await?;
            put_immutable(&self.bucket, &manifest, manifest_bytes, "application/json").await?;

            Ok(StagedArtifacts {
                revision,
                files: file_artifacts,
                manifest,
                zip,
                total_bytes,
            })
        })
    }

    fn read(
        &self,
        artifact: &ArtifactDescriptor,
        max_bytes: u64,
    ) -> impl Future<Output = anyhow::Result<Vec<u8>>> + Send {
        SendFuture::new(async move {
            let limit = max_bytes.min(encoding::MAX_ZIP_BYTES as u64);
            ensure!(
                artifact.byte_len <= limit,
                "artifact exceeds buffered read limit"
            );
            let object = self
                .bucket
                .get(artifact.key.as_storage_key())
                .execute()
                .await
                .map_err(|error| sdk_failure("read artifact", error))?
                .ok_or_else(|| anyhow!("published artifact is missing"))?;
            ensure!(
                object.size() == artifact.byte_len,
                "published artifact size mismatch"
            );
            let body = object
                .body()
                .ok_or_else(|| anyhow!("published artifact body is missing"))?;
            let bytes = body
                .bytes()
                .await
                .map_err(|error| sdk_failure("read artifact body", error))?;
            ensure!(
                bytes.len() as u64 == artifact.byte_len,
                "published artifact body size mismatch"
            );
            Ok(bytes)
        })
    }
}

fn descriptor(key: String, byte_len: usize) -> ArtifactDescriptor {
    ArtifactDescriptor {
        key: ArtifactKey::from_storage(key),
        byte_len: byte_len as u64,
    }
}

async fn put_immutable(
    bucket: &Bucket,
    artifact: &ArtifactDescriptor,
    bytes: Vec<u8>,
    content_type: &str,
) -> anyhow::Result<()> {
    ensure!(
        bytes.len() as u64 == artifact.byte_len,
        "staged artifact size mismatch"
    );
    let result = bucket
        .put(artifact.key.as_storage_key(), bytes)
        .only_if(Conditional {
            etag_does_not_match: Some("*".to_owned()),
            ..Default::default()
        })
        .http_metadata(HttpMetadata {
            content_type: Some(content_type.to_owned()),
            cache_control: Some("no-store".to_owned()),
            ..Default::default()
        })
        .execute()
        .await
        .map_err(|error| sdk_failure("write artifact", error))?;
    let object = result.ok_or_else(|| anyhow!("immutable artifact already exists"))?;
    ensure!(
        object.size() == artifact.byte_len,
        "stored artifact size mismatch"
    );
    Ok(())
}

fn sdk_failure(operation: &'static str, error: worker::Error) -> anyhow::Error {
    // The SDK's raw message can include private keys. Retain only its variant and operation;
    // converting here also drops non-Send JS errors before crossing the application boundary.
    let category: &'static str = error.into();
    anyhow!("R2 {operation} failed ({category})")
}
