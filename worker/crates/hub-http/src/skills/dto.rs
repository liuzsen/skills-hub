use base64::{Engine, engine::general_purpose::STANDARD};
use hub_application::skills::{SkillSummary, publish_skill, read_detail, read_text};
use hub_domain::{
    account::Actor,
    skill::{
        InvalidSkillFiles, MAX_FILE_BYTES, MAX_FILES, MAX_TOTAL_BYTES, SkillFile, SkillFiles,
        SkillId, SkillMetadata, SkillPath,
    },
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{DeserializeSeed, Error, SeqAccess, Visitor},
};
use std::{fmt, marker::PhantomData};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SummaryDto {
    id: String,
    name: String,
    description: String,
    tags: Vec<String>,
    updated_at: i64,
}

impl From<SkillSummary> for SummaryDto {
    fn from(summary: SkillSummary) -> Self {
        Self {
            id: summary.id.as_str().to_owned(),
            name: summary.metadata.name().to_owned(),
            description: summary.metadata.description().to_owned(),
            tags: summary.metadata.tags().to_vec(),
            updated_at: summary.updated_at,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ListDto {
    pub items: Vec<SummaryDto>,
    pub next_after: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FileDto {
    path: String,
    byte_len: u64,
    is_utf8: bool,
}
#[derive(Serialize)]
pub(super) struct ReadmeDto {
    path: String,
    content: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DetailDto {
    #[serde(flatten)]
    summary: SummaryDto,
    revision_id: String,
    files: Vec<FileDto>,
    readme: Option<ReadmeDto>,
}

impl From<read_detail::Output> for DetailDto {
    fn from(output: read_detail::Output) -> Self {
        Self {
            summary: output.summary.into(),
            revision_id: output.revision_id,
            files: output
                .files
                .into_iter()
                .map(|file| FileDto {
                    path: file.path.as_str().to_owned(),
                    byte_len: file.byte_len,
                    is_utf8: file.is_utf8,
                })
                .collect(),
            readme: output.readme.map(|readme| ReadmeDto {
                path: readme.path.as_str().to_owned(),
                content: readme.content,
            }),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TextDto {
    revision_id: String,
    path: String,
    content: String,
}
impl From<read_text::Output> for TextDto {
    fn from(output: read_text::Output) -> Self {
        Self {
            revision_id: output.revision_id,
            path: output.path.as_str().to_owned(),
            content: output.content,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PublishedDto {
    pub id: String,
    pub revision_id: String,
    pub updated_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PublishBody {
    id: Option<String>,
    name: String,
    description: String,
    #[serde(deserialize_with = "deserialize_tags")]
    tags: Vec<String>,
    expected_revision: Option<String>,
    #[serde(deserialize_with = "deserialize_files")]
    files: Vec<UploadFile>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UploadFile {
    path: String,
    content_base64: String,
}

fn deserialize_tags<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    deserialize_upload_sequence(deserializer, 10)
}

fn deserialize_files<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<UploadFile>, D::Error> {
    deserialize_upload_sequence(deserializer, MAX_FILES)
}

/// Local to upload DTOs: bound element allocation while Serde reads the sequence.
/// Never reserve from an untrusted size_hint or deserialize an over-limit element.
fn deserialize_upload_sequence<'de, D, T>(deserializer: D, limit: usize) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct UploadSequence<T> {
        limit: usize,
        element: PhantomData<T>,
    }
    impl<'de, T: Deserialize<'de>> Visitor<'de> for UploadSequence<T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                formatter,
                "an upload array of at most {} elements",
                self.limit
            )
        }

        fn visit_seq<S: SeqAccess<'de>>(self, mut sequence: S) -> Result<Vec<T>, S::Error> {
            let mut values = Vec::with_capacity(self.limit);
            for _ in 0..self.limit {
                match sequence.next_element()? {
                    Some(value) => values.push(value),
                    None => return Ok(values),
                }
            }
            // next_element_seed checks whether another element exists, then this seed
            // errors before parsing it (even if that element is large or nested).
            sequence.next_element_seed(RejectExtraUploadElement)?;
            Ok(values)
        }
    }
    deserializer.deserialize_seq(UploadSequence {
        limit,
        element: PhantomData,
    })
}

struct RejectExtraUploadElement;
impl<'de> DeserializeSeed<'de> for RejectExtraUploadElement {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, _deserializer: D) -> Result<(), D::Error> {
        Err(D::Error::custom("upload array exceeds element limit"))
    }
}

pub(super) struct InvalidUpload {
    pub status: u16,
    pub code: &'static str,
}
fn invalid(code: &'static str) -> InvalidUpload {
    InvalidUpload { status: 400, code }
}
fn too_large() -> InvalidUpload {
    InvalidUpload {
        status: 413,
        code: "payload_too_large",
    }
}

impl PublishBody {
    pub fn into_input(
        self,
        actor: Actor,
        update_id: Option<SkillId>,
    ) -> Result<publish_skill::Input, InvalidUpload> {
        let skill_id = match update_id {
            Some(id) => {
                if self.id.is_some() || self.expected_revision.is_none() {
                    return Err(invalid("invalid_request"));
                }
                id
            }
            None => {
                if self.expected_revision.is_some() {
                    return Err(invalid("invalid_request"));
                }
                SkillId::parse(self.id.ok_or_else(|| invalid("invalid_skill_id"))?)
                    .map_err(|_| invalid("invalid_skill_id"))?
            }
        };
        let metadata = SkillMetadata::new(self.name, self.description, self.tags)
            .map_err(|_| invalid("invalid_skill_metadata"))?;
        if self.files.len() > MAX_FILES {
            return Err(too_large());
        }
        let mut files = Vec::with_capacity(self.files.len());
        let mut total = 0usize;
        for file in self.files {
            let path = SkillPath::parse(file.path).map_err(|_| invalid("invalid_file_path"))?;
            if file.content_base64.len() > MAX_FILE_BYTES.div_ceil(3) * 4 {
                return Err(too_large());
            }
            let bytes = STANDARD
                .decode(file.content_base64)
                .map_err(|_| invalid("invalid_base64"))?;
            total = total.checked_add(bytes.len()).ok_or_else(too_large)?;
            if total > MAX_TOTAL_BYTES {
                return Err(too_large());
            }
            files.push(SkillFile::new(path, bytes).map_err(|_| too_large())?);
        }
        let files = SkillFiles::new(files).map_err(|error| match error {
            InvalidSkillFiles::TooManyFiles
            | InvalidSkillFiles::FileTooLarge { .. }
            | InvalidSkillFiles::TotalTooLarge => too_large(),
            InvalidSkillFiles::Empty => invalid("empty_file_set"),
            InvalidSkillFiles::DuplicatePath { .. } => invalid("duplicate_file_path"),
            InvalidSkillFiles::FileDirectoryConflict { .. } => invalid("file_directory_conflict"),
        })?;
        Ok(publish_skill::Input {
            actor,
            skill_id,
            metadata,
            expected_revision: self.expected_revision,
            files,
        })
    }
}
