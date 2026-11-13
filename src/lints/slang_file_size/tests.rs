use std::collections::BTreeSet;
use std::fs;

use super::*;
use crate::test_support::run_repo_lint;

fn check(lint: &SlangFileSize, files: &[(&str, String)]) -> Vec<LintError> {
    let d = tempfile::tempdir().unwrap();
    for (rel, text) in files {
        let p = d.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    let crates = BTreeSet::from(["alpha".to_string()]);
    run_repo_lint(lint, d.path(), &crates)
}

fn body(code_lines: usize) -> String {
    (0..code_lines).map(|i| format!("static const uint K{i} = {i}u;\n")).collect()
}

const BIG: &str = "mock/crates/alpha/shaders/big.slang";

#[test]
fn a_file_at_the_limit_is_clean_and_one_over_is_reported_once_at_line_one() {
    let lint = SlangFileSize::default();
    assert!(check(&lint, &[(BIG, body(500))]).is_empty());
    let found = check(&lint, &[(BIG, body(501))]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 1);
    assert_eq!(found[0].path.as_deref(), Some(BIG));
    assert!(found[0].message.contains("501"), "{}", found[0].message);
}

#[test]
fn blank_and_comment_lines_do_not_count() {
    let mut text = body(10);
    text.push_str(&"// note\n\n".repeat(600));
    assert!(check(&SlangFileSize::default(), &[(BIG, text)]).is_empty());
}

#[test]
fn the_limit_is_configurable() {
    let mut lint = SlangFileSize::default();
    lint.configure(&HashMap::from([(MAX_KEY.to_string(), "20".to_string())]));
    assert_eq!(check(&lint, &[(BIG, body(21))]).len(), 1);
    assert!(check(&lint, &[(BIG, body(20))]).is_empty());
}
