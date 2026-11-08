/// A relative, case-sensitive POSIX file path, used unchanged in the manifest and ZIP.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkillPath(String);

pub const MAX_PATH_BYTES: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidSkillPath {
    Empty,
    TooLong,
    UnsafeCharacter,
    UnsafeSegment,
}

impl SkillPath {
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidSkillPath> {
        let value = value.into();
        if value.is_empty() {
            return Err(InvalidSkillPath::Empty);
        }
        if value.len() > MAX_PATH_BYTES {
            return Err(InvalidSkillPath::TooLong);
        }
        // Colons also exclude drive prefixes and alternate data streams on extraction.
        if value
            .chars()
            .any(|ch| ch.is_control() || ch == '\\' || ch == ':')
        {
            return Err(InvalidSkillPath::UnsafeCharacter);
        }
        if value.split('/').any(|part| matches!(part, "" | "." | "..")) {
            return Err(InvalidSkillPath::UnsafeSegment);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
