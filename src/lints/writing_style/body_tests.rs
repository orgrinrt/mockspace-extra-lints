//! What a comment corpus is and is not checked for.
//!
//! The three structural checks are about a document's shape and each was
//! measured firing on ordinary rust before `Body` split them off. Every arm
//! here is one of those measurements, kept so the split cannot quietly close.
use super::scan::{fenced_lines, word_paired};
use super::{Body, check_text};

fn on(body: Body, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    check_text(text, "c", body, &mut out);
    out.into_iter().map(|e| e.message).collect()
}

fn comments(text: &str) -> Vec<String> {
    on(
        Body::Comments {
            lines: text.lines().count().max(40),
        },
        text,
    )
}

fn fired(found: &[String], needle: &str) -> bool {
    found.iter().any(|m| m.contains(needle))
}

#[test]
fn commented_out_code_is_not_exclamations_in_prose() {
    let found = comments(" debug_assert!(x > 0);\n todo!();\n if !ready {\n");
    assert!(!fired(&found, "exclamation"), "{found:?}");
}

#[test]
fn a_module_doc_opening_on_bullets_is_not_a_feature_dump() {
    let found = comments(" what this holds:\n - one\n - two\n - three\n - four\n");
    assert!(!fired(&found, "flat bulleted list"), "{found:?}");
}

#[test]
fn a_rustdoc_argument_list_is_not_a_glossary() {
    let found = comments(
        " - path: where it lands\n - name: what it is called\n - out: where findings go\n \
         - root: what paths hang off\n",
    );
    assert!(!fired(&found, "glossary table"), "{found:?}");
}

/// A label is one word. Four bullets whose left side of the colon is a
/// phrase are ordinary prose written as a list, not a glossary, and the
/// `!before.contains(' ')` half of `is_short_label` is the only thing that
/// tells the two apart. Deleting that clause leaves every other test in
/// this file green, which is what this one is here to stop.
#[test]
fn a_multi_word_label_is_prose_rather_than_a_glossary() {
    let phrases = "# t\n\n- the first thing: one\n- the second thing: two\n\
                   - the third thing: three\n- the fourth thing: four\n";
    let found = on(Body::Document, phrases);
    assert!(!fired(&found, "glossary table"), "{found:?}");

    // The control: the same four bullets with one-word labels do fire, so
    // the assertion above is about the labels rather than about the shape.
    let words = "# t\n\n- first: one\n- second: two\n- third: three\n- fourth: four\n";
    assert!(fired(&on(Body::Document, words), "glossary table"));
}

/// The same three fire on a document, which is what makes the split a split
/// rather than a deletion.
#[test]
fn a_document_still_gets_all_three() {
    let bullets = "# t\n\n- one\n- two\n- three\n- four\n";
    assert!(fired(&on(Body::Document, bullets), "flat bulleted list"));

    let excl = "# t\n\nwow! really! yes!\n";
    assert!(fired(&on(Body::Document, excl), "exclamation"));

    let labels = "# t\n\n- a: one\n- b: two\n- c: three\n- d: four\n";
    assert!(fired(&on(Body::Document, labels), "glossary table"));
}

/// The density is over the source, not over however many of its lines were
/// comments. Two em-dashes in a two-line corpus out of a hundred-line file
/// is under the threshold; the same two in a two-line file is over it.
#[test]
fn an_em_dash_density_is_measured_against_the_file() {
    let text = " one — two\n three — four\n";
    assert!(!fired(
        &on(
            Body::Comments {
                lines: 100,
            },
            text
        ),
        "em-dash density"
    ));
    assert!(fired(
        &on(
            Body::Comments {
                lines: 2,
            },
            text
        ),
        "em-dash density"
    ));
}

/// The silent zero, and the reason it is the worst shape available: nothing
/// in the output says the tail went unread.
#[test]
fn an_unterminated_fence_does_not_blank_what_follows() {
    let found = comments(" ```\n let x = 1;\n ```\n a **loud** claim\n");
    assert!(fired(&found, "inline emphasis"), "{found:?}");

    let unclosed = comments(" ```\n let x = 1;\n a **loud** claim\n");
    assert!(fired(&unclosed, "inline emphasis"), "{unclosed:?}");
}

#[test]
fn a_fence_pairs_in_order_and_an_odd_one_covers_nothing() {
    assert_eq!(fenced_lines("a\n```\nb\n```\nc\n"), vec![
        false, true, true, true, false
    ]);
    assert_eq!(fenced_lines("a\n```\nb\nc\n"), vec![
        false, false, false, false
    ]);
    assert_eq!(fenced_lines(""), Vec::<bool>::new());
}

/// The known cost of pairing by shape. One identifier carrying an
/// underscore does not pair; two on a line do, and that is a false positive
/// the check accepts rather than one it avoids.
#[test]
fn one_underscored_identifier_is_not_italic_and_two_are() {
    assert!(!word_paired("snake_case is a name", "_"));
    assert!(!word_paired("the _unused binding", "_"));
    assert!(word_paired("_private and public_", "_"));
    assert!(word_paired("the _start and the field_", "_"));
}
