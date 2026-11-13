//! Shader types are read in the same pass as Rust types, so a struct copied between two crates'
//! shaders is a near copy like one copied between two crates' Rust.

use mockspace_lint_rules::DeclaredType;

use super::tests::found;
use crate::slang::{SlangSource, declared_types, read};

fn shader(crate_name: &str, rel: &str, text: &str) -> SlangSource {
    SlangSource {
        rel:        format!("mock/crates/{crate_name}/{rel}"),
        crate_name: crate_name.to_string(),
        text:       text.to_string(),
        file:       read(text),
    }
}

fn types(sources: &[SlangSource]) -> Vec<DeclaredType> {
    declared_types(sources)
}

const FRAME: &str = "public struct Frame { public float4x4 view; public uint4 counts; public float time; };\n";

#[test]
fn a_shader_struct_copied_with_one_member_more_is_a_near_copy() {
    let t = types(&[
        shader("alpha", "shaders/frame.slang", FRAME),
        shader(
            "beta",
            "shaders/tick.slang",
            "public struct Tick { public float4x4 view; public uint4 counts; };\n",
        ),
    ]);
    let found = found(&t);
    assert_eq!(found.near_copies.len(), 1, "{t:?}");
}

#[test]
fn two_unrelated_shader_structs_are_not_near_copies() {
    let t = types(&[
        shader("alpha", "shaders/frame.slang", FRAME),
        shader("beta", "shaders/ray.slang", "struct Ray { float3 origin; float3 dir; };\n"),
    ]);
    assert!(found(&t).near_copies.is_empty());
}

#[test]
fn a_probe_struct_is_not_read() {
    let t = types(&[
        shader("alpha", "shaders/frame.slang", FRAME),
        shader("beta", "src/tests/probe/frame.slang", FRAME.replace("Frame", "Probe").as_str()),
    ]);
    assert_eq!(t.len(), 1, "{t:?}");
}
