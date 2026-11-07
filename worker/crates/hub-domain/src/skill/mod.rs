mod metadata;
mod path;

pub use metadata::{InvalidSkillId, InvalidSkillMetadata, SkillId, SkillMetadata};
pub use path::{InvalidSkillPath, MAX_PATH_BYTES, SkillPath};

use std::collections::BTreeSet;

pub const MAX_FILES: usize = 200;
pub const MAX_FILE_BYTES: usize = 1024 * 1024;
pub const MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;

pub struct SkillFile {
    path: SkillPath,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvalidSkillFiles {
    Empty,
    TooManyFiles,
    FileTooLarge { path: SkillPath },
    TotalTooLarge,
    DuplicatePath { path: SkillPath },
    FileDirectoryConflict { path: SkillPath },
}

impl SkillFile {
    pub fn new(path: SkillPath, bytes: Vec<u8>) -> Result<Self, InvalidSkillFiles> {
        if bytes.len() > MAX_FILE_BYTES {
            return Err(InvalidSkillFiles::FileTooLarge { path });
        }
        Ok(Self { path, bytes })
    }

    pub fn path(&self) -> &SkillPath {
        &self.path
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn is_utf8(&self) -> bool {
        std::str::from_utf8(&self.bytes).is_ok()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A nonempty, bounded file set with no duplicate names or file/directory conflicts.
/// Sorted by POSIX path so manifests and archives have the same stable order.
pub struct SkillFiles {
    files: Vec<SkillFile>,
    total_bytes: usize,
}

impl SkillFiles {
    pub fn new(mut files: Vec<SkillFile>) -> Result<Self, InvalidSkillFiles> {
        if files.is_empty() {
            return Err(InvalidSkillFiles::Empty);
        }
        if files.len() > MAX_FILES {
            return Err(InvalidSkillFiles::TooManyFiles);
        }
        let mut total_bytes = 0usize;
        let mut names = BTreeSet::new();
        for file in &files {
            total_bytes = total_bytes
                .checked_add(file.bytes.len())
                .filter(|total| *total <= MAX_TOTAL_BYTES)
                .ok_or(InvalidSkillFiles::TotalTooLarge)?;
            if !names.insert(file.path.as_str()) {
                return Err(InvalidSkillFiles::DuplicatePath {
                    path: file.path.clone(),
                });
            }
        }
        // Check every ancestor, not only adjacent sorted names: a, a-b, a/file.
        for file in &files {
            let name = file.path.as_str();
            for (index, _) in name.match_indices('/') {
                if names.contains(&name[..index]) {
                    return Err(InvalidSkillFiles::FileDirectoryConflict {
                        path: file.path.clone(),
                    });
                }
            }
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(Self { files, total_bytes })
    }

    pub fn files(&self) -> &[SkillFile] {
        &self.files
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn into_files(self) -> Vec<SkillFile> {
        self.files
    }
}

#[cfg(test)]
#[path = "tests/paths.rs"]
mod tests;
