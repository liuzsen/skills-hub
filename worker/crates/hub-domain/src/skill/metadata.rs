#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SkillId(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidSkillId;

impl SkillId {
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidSkillId> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
            })
            && value.as_bytes()[0].is_ascii_alphanumeric();
        if valid {
            Ok(Self(value))
        } else {
            Err(InvalidSkillId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillMetadata {
    name: String,
    description: String,
    tags: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidSkillMetadata;

impl SkillMetadata {
    pub fn new(
        name: String,
        description: String,
        tags: Vec<String>,
    ) -> Result<Self, InvalidSkillMetadata> {
        fn text(value: &str, max: usize) -> bool {
            !value.is_empty()
                && value.len() <= max
                && value.trim() == value
                && !value.chars().any(char::is_control)
        }
        if !text(&name, 240)
            || !text(&description, 1024)
            || tags.len() > 10
            || tags.iter().any(|tag| !text(tag, 64))
        {
            return Err(InvalidSkillMetadata);
        }
        let mut sorted = tags.clone();
        sorted.sort();
        sorted.dedup();
        if sorted.len() != tags.len() {
            return Err(InvalidSkillMetadata);
        }
        Ok(Self {
            name,
            description,
            tags,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
}
