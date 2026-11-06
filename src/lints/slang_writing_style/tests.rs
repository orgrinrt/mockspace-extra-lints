use super::*;
use crate::slang::read;

fn on(text: &str) -> Vec<LintError> {
    judge(&SlangSource {
        rel:        "mock/crates/alpha/shaders/x.slang".to_string(),
        crate_name: "alpha".to_string(),
        text:       text.to_string(),
        file:       read(text),
    })
}

#[test]
fn plain_comments_are_clean() {
    assert!(on("// where the ray stands\nfloat a;\n").is_empty());
}

#[test]
fn emphasis_in_a_line_comment_is_reported_at_its_line_under_this_lint() {
    let found = on("float a;\n// a **loud** claim\nfloat b;\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 2);
    assert_eq!(found[0].lint_name, NAME);
    assert_eq!(found[0].path.as_deref(), Some("mock/crates/alpha/shaders/x.slang"));
}

#[test]
fn emphasis_in_a_block_comment_is_reported_at_its_own_line() {
    let found = on("/* first\n   a **loud** claim */\nfloat a;\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 2);
}

#[test]
fn a_string_that_looks_like_emphasis_is_not_a_comment() {
    assert!(on("static const string S = \"// **loud**\";\n").is_empty());
}
