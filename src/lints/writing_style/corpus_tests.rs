use super::{check_rust_comments, comment_corpus};

fn lines(source: &str) -> Vec<usize> {
    comment_corpus(source).1
}

fn text(source: &str) -> String {
    comment_corpus(source).0
}

#[test]
fn all_three_comment_forms_are_collected() {
    let src = "//! module\n/// item\n// plain\nlet x = 1;\n";
    assert_eq!(text(src), " module\n item\n plain\n");
    assert_eq!(lines(src), vec![1, 2, 3]);
}

#[test]
fn the_map_names_the_source_line_not_the_corpus_line() {
    let src = "fn a() {}\nfn b() {}\n// third line\nfn c() {}\n// fifth line\n";
    assert_eq!(lines(src), vec![3, 5]);
}

#[test]
fn indentation_does_not_hide_a_comment() {
    assert_eq!(lines("fn f() {\n        // deep\n}\n"), vec![2]);
}

#[test]
fn a_doc_form_is_matched_before_the_bare_one() {
    // `//` is a prefix of `///`, so the wrong order would strip two slashes
    // and leave a stray one at the front of every doc line.
    assert_eq!(text("/// item\n"), " item\n");
    assert_eq!(text("//! module\n"), " module\n");
}

#[test]
fn a_four_slash_divider_is_still_a_comment() {
    assert_eq!(text("//// divider\n"), "/ divider\n");
}

#[test]
fn a_file_with_no_comments_yields_nothing() {
    assert!(text("fn main() {}\n").is_empty());
    assert!(lines("fn main() {}\n").is_empty());
    assert!(text("").is_empty());
}

#[test]
fn a_slash_pair_inside_a_string_is_not_collected() {
    // The scan is line-leading, so a literal cannot masquerade as a comment.
    assert!(text("let s = \"// not a comment\";\n").is_empty());
}

#[test]
fn a_finding_carries_the_file_and_the_source_line() {
    let src = "fn a() {}\nfn b() {}\n// a **loud** claim\n";
    let mut out = Vec::new();
    check_rust_comments("src/thing.rs", src, "crate-x", &mut out);
    let emphasis: Vec<_> = out
        .iter()
        .filter(|e| e.message.starts_with("inline emphasis"))
        .collect();
    assert_eq!(emphasis.len(), 1);
    assert_eq!(emphasis[0].line, 3);
    assert_eq!(emphasis[0].path.as_deref(), Some("src/thing.rs"));
    assert_eq!(emphasis[0].crate_name, "crate-x");
}

#[test]
fn a_clean_file_reports_nothing() {
    let mut out = Vec::new();
    check_rust_comments(
        "src/clean.rs",
        "// a claim carrying its weight in the words\nfn f() {}\n",
        "crate-x",
        &mut out,
    );
    assert!(out.is_empty(), "unexpected findings: {out:?}");
}

#[test]
fn a_file_with_no_comments_is_skipped_entirely() {
    let mut out = Vec::new();
    check_rust_comments("src/empty.rs", "fn main() {}\n", "crate-x", &mut out);
    assert!(out.is_empty());
}

#[test]
#[ignore = "catalogue: a trailing `// why` comment is not collected, only a \
            line-leading one, so the commonest inline-comment position is \
            unchecked. Needs string and char literals stripped first, or a \
            `//` inside one reads as a comment"]
fn a_trailing_comment_is_collected() {
    assert_eq!(lines("let x = 1; // a **loud** claim\n"), vec![1]);
}
