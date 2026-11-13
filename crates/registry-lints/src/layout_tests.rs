use std::path::Path;

use super::*;

fn layout(mock: &str, content: &[&str]) -> Layout {
    Layout::new(mock, ContentDirs::new(content))
}

#[test]
fn the_mock_directory_is_read_from_its_place_under_the_root() {
    assert_eq!(mock_rel(Path::new("/r/linux"), Path::new("/r")), "linux");
    assert_eq!(mock_rel(Path::new("/r"), Path::new("/r")), "");
    let l = Layout::of(Path::new("/r/linux"), Path::new("/r"), &ContentDirs::default());
    assert_eq!(l.mock(), "linux");
    assert_eq!(l.in_mock("registry"), "linux/registry");
    assert_eq!(layout("", &[]).in_mock("registry"), "registry");
}

#[test]
fn a_path_is_under_the_mock_directory_only_past_its_whole_name() {
    let l = layout("linux", &[]);
    assert_eq!(l.under_mock("linux/crates/a/src/lib.rs"), Some("crates/a/src/lib.rs"));
    assert_eq!(l.under_mock("linuxes/crates/a.rs"), None);
    assert_eq!(l.under_mock("mock/crates/a.rs"), None);
    assert_eq!(layout("", &[]).under_mock("crates/a.rs"), Some("crates/a.rs"));
}

#[test]
fn a_path_is_in_content_only_under_a_directory_the_project_named() {
    let l = layout("mock", &["content", "packs/"]);
    assert_eq!(l.in_content("content/base/x.toml"), Some(("content", "base/x.toml")));
    assert_eq!(l.in_content("packs/a.toml"), Some(("packs", "a.toml")));
    assert_eq!(l.in_content("contents/a.toml"), None);
    // THE CONTROL: with none named, the same path is no content.
    assert_eq!(layout("mock", &[]).in_content("content/base/x.toml"), None);
}

#[test]
fn a_lint_takes_its_content_directories_comma_separated_and_a_tool_as_a_list() {
    let mut lint = ContentDirs::default();
    lint.configure_lint(&HashMap::from([(CONTENT_DIRS.to_owned(), " content, packs/ ,".to_owned())]));
    assert_eq!(lint.dirs(), ["content", "packs"]);
    let mut untouched = ContentDirs::new(&["content"]);
    untouched.configure_lint(&HashMap::new());
    assert_eq!(untouched.dirs(), ["content"]);
}

#[test]
fn a_tool_takes_its_content_directories_as_a_list() {
    let mut params = ToolParams::new();
    params.set(CONTENT_DIRS, vec!["content".to_owned(), "packs/".to_owned()]);
    let mut tool = ContentDirs::default();
    tool.configure_tool(&params);
    assert_eq!(tool.dirs(), ["content", "packs"]);
}
