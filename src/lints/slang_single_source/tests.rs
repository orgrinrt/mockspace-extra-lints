use super::*;
use crate::slang::read;

fn at(crate_name: &str, rel: &str, text: &str) -> SlangSource {
    SlangSource {
        rel:        format!("mock/crates/{crate_name}/{rel}"),
        crate_name: crate_name.to_string(),
        text:       text.to_string(),
        file:       read(text),
    }
}

const FRAME: &str = "module a;\npublic struct Frame { public float4x4 view; public uint4 counts; };\n";

#[test]
fn one_declaration_is_clean() {
    assert!(judge(&[at("alpha", "shaders/a.slang", FRAME)]).is_empty());
}

#[test]
fn the_same_struct_in_a_second_crate_is_reported_at_the_copy_and_names_the_source() {
    let found = judge(&[
        at("alpha", "shaders/a.slang", FRAME),
        at("beta", "shaders/b.slang", "module b;\nstruct Frame { float4x4 view; uint4 counts; };\n"),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path.as_deref(), Some("mock/crates/beta/shaders/b.slang"));
    assert_eq!(found[0].line, 2);
    assert!(found[0].message.contains("alpha"), "{}", found[0].message);
}

#[test]
fn one_name_over_two_shapes_is_not_this_lints() {
    let found = judge(&[
        at("alpha", "shaders/a.slang", FRAME),
        at("beta", "shaders/b.slang", "struct Frame { float4x4 view; };\n"),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_copy_in_the_same_crate_is_left_to_the_compiler() {
    let found = judge(&[
        at("alpha", "shaders/a.slang", FRAME),
        at("alpha", "shaders/c.slang", FRAME),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_marked_copy_and_a_probe_are_let_through() {
    let marked = "struct Frame { float4x4 view; uint4 counts; }; // lint:allow(slang-single-source)\n";
    let found = judge(&[
        at("alpha", "shaders/a.slang", FRAME),
        at("beta", "shaders/b.slang", marked),
        at("beta", "src/tests/probe/p.slang", FRAME),
    ]);
    assert!(found.is_empty(), "{found:?}");
}
