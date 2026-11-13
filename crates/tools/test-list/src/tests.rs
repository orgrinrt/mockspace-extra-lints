use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use super::mask::masked;
use super::{Found, Place, list, render, tests_in};

fn found(line: usize, name: &str, ignore: Option<&str>) -> Found {
    Found {
        line,
        name: name.to_string(),
        ignore: ignore.map(str::to_string),
    }
}

fn scratch(tag: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let d = std::env::temp_dir().join(format!(
        "kaski-test-list-{tag}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("a scratch directory");
    d
}

fn write(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().expect("a parent")).expect("a directory");
    std::fs::write(p, body).expect("a file");
}

#[test]
fn a_test_is_named_with_the_line_of_its_fn_and_a_plain_function_is_not_one() {
    let source =
        "fn helper() {}\n\n#[test]\nfn it_holds() {}\n\n#[cfg(test)]\nfn only_in_tests() {}\n";
    assert_eq!(tests_in(source), [found(4, "it_holds", None)]);
}

#[test]
fn the_attributes_above_a_test_are_read_in_any_order_and_across_doc_comments() {
    let source = "#[ignore = \"catalogue: slow; tracked #t_1\"]\n/// Why.\n#[test]\n#[should_panic]\npub(crate) async fn late() {}\n";
    assert_eq!(tests_in(source), [found(
        5,
        "late",
        Some("catalogue: slow; tracked #t_1")
    )]);
}

#[test]
fn an_ignore_says_its_reason_on_one_line_and_a_bare_one_says_nothing() {
    let source = "#[test]\n#[ignore = \"catalogue: a gap\n    that runs on; tracked #t_2\"]\nfn a() {}\n#[test]\n#[ignore]\nfn b() {}\n";
    assert_eq!(tests_in(source), [
        found(4, "a", Some("catalogue: a gap that runs on; tracked #t_2")),
        found(7, "b", Some("")),
    ]);
}

#[test]
fn an_attribute_whose_path_ends_in_test_makes_a_test_and_one_that_only_holds_the_word_does_not() {
    let source = "#[tokio::test]\nasync fn a() {}\n#[wasm_bindgen_test]\nfn b() {}\n#[cfg(test)]\nfn c() {}\n#[test_helper]\nfn d() {}\n";
    let names: Vec<String> = tests_in(source).into_iter().map(|f| f.name).collect();
    assert_eq!(names, ["a", "b"]);
}

#[test]
fn a_test_written_in_a_string_or_a_comment_is_no_test() {
    let source = "const PLANTED: &str = \"\n#[test]\nfn planted() {}\n\";\nconst RAW: &str = r#\"\n#[test]\nfn raw() {}\n\"#;\n// #[test]\n// fn commented() {}\n/* #[test]\nfn blocked() {} */\n#[test]\nfn real() {}\n";
    assert_eq!(tests_in(source), [found(14, "real", None)]);
}

#[test]
fn a_quote_in_a_character_literal_or_a_lifetime_opens_no_string() {
    let source = "fn a<'a>(x: &'a str) -> char { '\"' }\n#[test]\nfn after() { let _ = '\\''; }\n#[test]\nfn last() {}\n";
    let names: Vec<String> = tests_in(source).into_iter().map(|f| f.name).collect();
    assert_eq!(names, ["after", "last"]);
}

#[test]
fn a_test_in_a_macro_call_is_listed_and_a_template_carries_the_fragment_as_its_name() {
    let source = "proptest! {\n    #[test]\n    fn sampled(x in 0u8..4) {}\n}\nmacro_rules! suite {\n    ($name:ident) => {\n        #[test]\n        fn $name() {}\n    };\n}\n";
    assert_eq!(tests_in(source), [
        found(3, "sampled", None),
        found(8, "$name", None)
    ]);
}

#[test]
fn an_attribute_on_something_else_does_not_reach_the_function_after_it() {
    let source = "#[test]\nstruct NotAFunction;\nfn after() {}\n";
    assert!(tests_in(source).is_empty());
}

#[test]
fn the_mask_keeps_every_offset_and_line() {
    let source = "let s = \"a // b\"; // c\nlet é = 'é';\n";
    let m = masked(source);
    assert_eq!(m.len(), source.len());
    assert_eq!(m.lines().count(), source.lines().count());
    assert!(m.starts_with("let s = \"      \";"), "{m}");
}

#[test]
fn a_file_is_placed_by_the_first_directory_of_its_path_in_the_crate() {
    assert_eq!(Place::of(Path::new("src/a/tests.rs")), Place::Src);
    assert_eq!(Place::of(Path::new("tests/main.rs")), Place::Tests);
    assert_eq!(Place::of(Path::new("benches/b.rs")), Place::Other);
    assert_eq!(Place::of(Path::new("build.rs")), Place::Other);
}

#[test]
fn the_list_walks_every_crate_in_order_and_an_argument_narrows_it() {
    let d = scratch("walk");
    write(
        &d,
        "kaski-b/src/lib.rs",
        "#[cfg(test)]\nmod tests {\n    #[test]\n    fn inline() {}\n}\n",
    );
    write(
        &d,
        "kaski-a/tests/main.rs",
        "#[test]\n#[ignore = \"catalogue: long; tracked #t_3\"]\nfn outside() {}\n",
    );
    write(
        &d,
        "kaski-a/target/debug/gen.rs",
        "#[test]\nfn built() {}\n",
    );
    write(&d, "kaski-a/src/lib.rs", "pub fn nothing() {}\n");

    let all = list(&d, None);
    let seen: Vec<(&str, &str, Place)> = all
        .iter()
        .map(|r| (r.crate_name.as_str(), r.found.name.as_str(), r.place))
        .collect();
    assert_eq!(seen, [
        ("kaski-a", "outside", Place::Tests),
        ("kaski-b", "inline", Place::Src)
    ]);

    let text = render(&all);
    assert!(
        text.contains(
            "kaski-a\ttests/main.rs:3\toutside\ttests\tignored: catalogue: long; tracked #t_3\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("kaski-b\tsrc/lib.rs:4\tinline\tsrc\t-\n"),
        "{text}"
    );
    assert!(
        text.ends_with("2 tests in 2 crates: 1 under src, 1 under tests, 0 elsewhere; 1 ignored\n"),
        "{text}"
    );

    let one = list(&d, Some("kaski-b"));
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].crate_name, "kaski-b");
}

#[test]
fn a_crates_directory_that_is_not_there_lists_nothing() {
    let d = scratch("absent");
    assert!(list(&d.join("nope"), None).is_empty());
}
