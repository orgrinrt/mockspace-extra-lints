use super::super::*;

/// The schema's shape, near enough: two namespaces, a field block of each
/// kind, and the nesting that decides which namespace a field belongs to.
const SCHEMA: &str = r#"
[[registry.namespace]]
key = "slot"
title = "Slots"

[[registry.namespace.field]]
name = "use"
type = "string"
required = true
description = "what the person is doing"

[[registry.namespace.field]]
name = "source"
type = "string"
required = true
visibility = "internal"
description = "held rather than rendered"

[[registry.namespace.field]]
name = "keywords"
type = "string[]"
required = false
description = "search words"

[[registry.namespace]]
key = "ruling"
title = "Rulings"

[[registry.namespace.field]]
name = "kind"
type = "string"
required = true
description = "a label"
values = ["intent", "ruling", "refusal"]

[[registry.namespace.field]]
name = "topic"
type = "topic"
required = true
description = "a relation"

[[registry.namespace.field]]
name = "says"
type = "string"
required = true
description = "what op stated"
"#;

#[test]
fn the_field_list_comes_out_of_the_schema() {
    // The defect: a hand-written seven, against twenty-six declared. Here
    // the two statement fields are found and nothing else is.
    let f = statement_fields(SCHEMA);
    assert_eq!(f["slot"], BTreeSet::from(["use".to_string()]));
    assert_eq!(f["ruling"], BTreeSet::from(["says".to_string()]));
}

#[test]
fn an_internal_field_is_not_a_statement() {
    // `quote` is op's exact words, which is the one thing here nobody may
    // edit, so a tool for finding statements that will need editing must not
    // read it. `source` and `key` are the same call and all three carry the
    // marker, so none of them is named in this file.
    let f = statement_fields(SCHEMA);
    assert!(!f["slot"].contains("source"));
    // The control, and it is what makes the marker the right test rather
    // than the field's name: a string field without it is read.
    assert!(f["slot"].contains("use"));
}

#[test]
fn a_label_and_a_relation_and_a_word_list_are_not_statements() {
    let f = statement_fields(SCHEMA);
    assert!(
        !f["ruling"].contains("kind"),
        "a closed enumeration is a label"
    );
    assert!(!f["ruling"].contains("topic"), "a relation names a row");
    assert!(
        !f["slot"].contains("keywords"),
        "a string[] is search words"
    );
}

#[test]
fn a_field_block_belongs_to_the_namespace_whose_header_last_opened() {
    // The nesting, and the reason a scanner can get this wrong quietly: the
    // last field of one namespace sits immediately above the next
    // namespace's header, so reading a field as belonging to the nearest
    // `key` below it puts `says` under `slot`.
    let f = statement_fields(SCHEMA);
    assert!(f["ruling"].contains("says"));
    assert!(!f["slot"].contains("says"));
}
