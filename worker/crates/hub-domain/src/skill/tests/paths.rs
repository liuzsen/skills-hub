use super::{InvalidSkillFiles, SkillFile, SkillFiles, SkillPath};

#[test]
fn preserves_valid_posix_names_without_normalization() {
    for name in [
        "SKILL.md",
        "资料/介绍.md",
        ".config/rules",
        "assets/图标.png",
        "a.../b",
    ] {
        assert_eq!(SkillPath::parse(name).unwrap().as_str(), name);
    }
}

#[test]
fn rejects_ambiguous_or_unsafe_paths() {
    for name in [
        "",
        "/root",
        "//root",
        "a/",
        "a//b",
        ".",
        "..",
        "a/./b",
        "a/../b",
        "a\\b",
        "C:/file",
        "file:stream",
        "a\0b",
        "a\nb",
        "a\u{7f}b",
    ] {
        assert!(SkillPath::parse(name).is_err(), "accepted {name:?}");
    }
    assert!(SkillPath::parse("a".repeat(1025)).is_err());
    assert!(SkillPath::parse("字".repeat(342)).is_err());
}

fn file(name: &str) -> SkillFile {
    SkillFile::new(SkillPath::parse(name).unwrap(), vec![]).unwrap()
}

#[test]
fn catches_duplicate_and_nonadjacent_ancestor_conflicts() {
    assert!(matches!(
        SkillFiles::new(vec![file("a/b"), file("a/b")]),
        Err(InvalidSkillFiles::DuplicatePath { .. })
    ));
    assert!(matches!(
        SkillFiles::new(vec![file("a"), file("a-b"), file("a/b")]),
        Err(InvalidSkillFiles::FileDirectoryConflict { .. })
    ));
    assert!(matches!(
        SkillFiles::new(vec![file("a/b/c"), file("a/b")]),
        Err(InvalidSkillFiles::FileDirectoryConflict { .. })
    ));
    assert!(SkillFiles::new(vec![file("a-b"), file("a/b"), file("A")]).is_ok());
}
