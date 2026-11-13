use super::*;

const TAIL_COMMON: &str = r#"// what every pass shares
module tail_common;

import lattice;
import a.b;

public struct TailFrame
{
    public float4x4 inverse_view_proj; // clip to world
    public uint4 counts;
};

// where the ray stands
public float3 unproject(TailFrame frame, uint2 pixel, float depth)
{
    float2 ndc = (float2(pixel) + 0.5) / frame.target.xy * 2.0 - 1.0;
    return float3(ndc, depth);
}

float along(float3 near, in float3 far, float3 p = float3(0.0)) { return 1.0; }
"#;

fn item<'a>(file: &'a SlangFile, name: &str) -> &'a Item {
    file.items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no item {name} in {:?}", file.items))
}

#[test]
fn a_module_header_and_its_imports_are_read_with_their_lines() {
    let f = read(TAIL_COMMON);
    assert_eq!(f.module, Some(("tail_common".to_string(), 2)));
    let got: Vec<_> = f.imports.iter().map(|i| (i.name.as_str(), i.line, i.form)).collect();
    assert_eq!(got, [("lattice", 4, ImportForm::Import), ("a.b", 5, ImportForm::Import)]);
}

#[test]
fn a_function_gives_its_name_visibility_parameter_types_and_return_type() {
    let f = read(TAIL_COMMON);
    let u = item(&f, "unproject");
    assert_eq!(u.kind, ItemKind::Func);
    assert_eq!(u.visibility, Visibility::Public);
    assert_eq!(u.params, ["TailFrame", "uint2", "float"]);
    assert_eq!(u.ret, "float3");
    assert_eq!(u.line, 14);
    let a = item(&f, "along");
    assert_eq!(a.visibility, Visibility::Unmarked);
    assert!(!a.exported());
    assert_eq!(a.params, ["float3", "in float3", "float3"]);
}

#[test]
fn a_struct_is_an_item_and_its_members_are_not() {
    let f = read(TAIL_COMMON);
    let names: Vec<_> = f.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["TailFrame", "unproject", "along"]);
    assert_eq!(item(&f, "TailFrame").kind, ItemKind::Struct);
}

#[test]
fn what_a_module_offers_is_what_is_marked_public() {
    let f = read(TAIL_COMMON);
    let exported: Vec<_> = f.exports().map(|i| i.name.as_str()).collect();
    assert_eq!(exported, ["TailFrame", "unproject"]);
}

#[test]
fn a_comment_or_a_string_that_names_a_declaration_declares_nothing() {
    let f = read(
        "// import hidden;\n/* public float ghost(float a) { return a; }\n   import also_hidden; */\nstatic const string S = \"import quoted;\";\nfloat real(float a) { return a; }\n",
    );
    assert!(f.imports.is_empty(), "{:?}", f.imports);
    let names: Vec<_> = f.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["S", "real"]);
}

#[test]
fn an_entry_is_a_function_under_a_shader_attribute() {
    let f = read(
        "[shader(\"compute\")]\n[numthreads(8, 8, 1)]\nvoid cs(uint3 id : SV_DispatchThreadID)\n{\n}\n\n[shader(\"fragment\")]\nfloat4 fs(float2 uv : TEXCOORD0) : SV_Target\n{\n    return float4(uv, 0, 1);\n}\n\nfloat plain(float a) { return a; }\n",
    );
    let got: Vec<_> = f.entries().map(|(i, s)| (i.name.as_str(), s)).collect();
    assert_eq!(got, [("cs", "compute"), ("fs", "fragment")]);
    assert_eq!(item(&f, "cs").params, ["uint3"]);
    assert_eq!(item(&f, "fs").ret, "float4");
    assert!(item(&f, "plain").stage().is_none());
}

#[test]
fn a_constant_a_resource_and_a_register_annotation_are_not_functions() {
    let f = read(
        "static const float K[2] = { 1.0, 2.0 };\npublic static const uint N = 4u;\nRWStructuredBuffer<uint> out_buf;\nTexture2D<float4> tex : register(t0);\ngroupshared float cache[64];\n",
    );
    let got: Vec<_> = f.items.iter().map(|i| (i.name.as_str(), i.kind)).collect();
    assert_eq!(
        got,
        [
            ("K", ItemKind::Const),
            ("N", ItemKind::Const),
            ("out_buf", ItemKind::Global),
            ("tex", ItemKind::Global),
            ("cache", ItemKind::Global),
        ]
    );
    assert!(item(&f, "N").exported());
}

#[test]
fn types_interfaces_enums_aliases_and_namespaces_are_read() {
    let f = read(
        "interface IShade { float4 shade(float3 n); }\nenum Mode : uint { A, B }\ntypealias Colour = float3;\nextension float3 : IShade { float4 shade(float3 n) { return 0; } }\nnamespace m { public float inner(float a) { return a; } }\nstruct G<T : IShade> : IShade { T t; };\n",
    );
    let got: Vec<_> = f.items.iter().map(|i| (i.name.as_str(), i.kind)).collect();
    assert_eq!(
        got,
        [
            ("IShade", ItemKind::Interface),
            ("Mode", ItemKind::Enum),
            ("Colour", ItemKind::Alias),
            ("float3", ItemKind::Extension),
            ("inner", ItemKind::Func),
            ("G", ItemKind::Struct),
        ]
    );
}

fn members(file: &SlangFile, name: &str) -> Vec<(String, String)> {
    item(file, name).members.iter().map(|m| (m.name.clone(), m.body.clone())).collect()
}

#[test]
fn a_struct_gives_its_fields_and_methods_an_enum_its_variants_and_an_interface_its_methods() {
    let f = read(
        "public struct P<T : IShade> : IShade\n{\n    public float4 colour : SV_Target;\n    static const uint N = 4u;\n    float2 uv;\n    float shade(float3 n, in float k) { float a = n.x; return a * k; }\n    __init(float a) { }\n    float after;\n};\nenum Mode : uint { A = 1, B, C = 4 }\ninterface IShade { float4 shade(float3 n); void bump(); }\n",
    );
    assert_eq!(
        members(&f, "P"),
        [
            ("colour".to_string(), "float4".to_string()),
            ("N".to_string(), "uint".to_string()),
            ("uv".to_string(), "float2".to_string()),
            ("shade".to_string(), "(float3, in float) float".to_string()),
            ("__init".to_string(), "(float)".to_string()),
            ("after".to_string(), "float".to_string()),
        ]
    );
    assert_eq!(item(&f, "P").generics, "T:IShade");
    assert_eq!(
        members(&f, "Mode"),
        [
            ("A".to_string(), "1".to_string()),
            ("B".to_string(), String::new()),
            ("C".to_string(), "4".to_string()),
        ]
    );
    assert_eq!(
        members(&f, "IShade"),
        [
            ("shade".to_string(), "(float3) float4".to_string()),
            ("bump".to_string(), "() void".to_string()),
        ]
    );
}

#[test]
fn a_generic_function_is_named_by_its_name_and_not_its_parameter_list() {
    let f = read("public T pick<T : IShade>(T a, T b) { return a; }\n");
    let p = item(&f, "pick");
    assert_eq!(p.params, ["T", "T"]);
    assert_eq!(p.ret, "T");
}

#[test]
fn comments_keep_their_lines_and_a_doc_comment_is_marked() {
    let f = read("// plain\n/// doc\n//! inner\n/* block\n * second */\nint x;\n/** doc block */\n");
    let got: Vec<_> = f.comments.iter().map(|c| (c.line, c.text.as_str(), c.doc)).collect();
    assert_eq!(
        got,
        [
            (1, "plain", false),
            (2, "doc", true),
            (3, "inner", true),
            (4, "block", false),
            (5, "second", false),
            (7, "doc block", true),
        ]
    );
}

#[test]
fn code_lines_are_the_lines_that_are_neither_blank_nor_only_a_comment() {
    let f = read("// c\n\nfloat a;\n  /* b */\nfloat b; // tail\n/*\n multi\n*/\n#define X 1\n");
    assert_eq!(f.code_lines, 3);
}

#[test]
fn a_preprocessor_line_is_kept_whole_and_its_braces_open_no_scope() {
    let f = read("#define BODY { \\\n  1 }\n#include \"x.slang\"\nfloat f(float a) { return a; }\n");
    assert_eq!(f.directives.len(), 2);
    assert_eq!(f.directives[1].text, "#include \"x.slang\"");
    assert_eq!(f.directives[1].line, 3);
    assert_eq!(f.items.len(), 1);
}

#[test]
fn an_include_and_an_implementing_are_imports_of_their_own_form() {
    let f = read("implementing parent;\n__include \"part.slang\";\nimport child;\n");
    let got: Vec<_> = f.imports.iter().map(|i| (i.name.as_str(), i.form)).collect();
    assert_eq!(
        got,
        [
            ("parent", ImportForm::Implementing),
            ("part.slang", ImportForm::Include),
            ("child", ImportForm::Import),
        ]
    );
}

#[test]
fn an_unclosed_body_or_comment_ends_the_read_without_a_panic() {
    let _ = read("float f(float a) { return a;");
    let _ = read("/* never closed");
    let _ = read("static const string S = \"open");
    let _ = read("struct S { float a;");
    let _ = read("[shader(");
    let _ = read(")))}}}");
}

#[test]
fn a_stem_and_where_a_file_sits_say_whether_it_ships() {
    let source = |rel: &str| SlangSource {
        rel:        rel.to_string(),
        crate_name: "c".to_string(),
        text:       String::new(),
        file:       read(""),
    };
    let shipped = source("mock/crates/c/shaders/light.slang");
    assert!(shipped.shipped());
    assert_eq!(shipped.stem(), "light");
    assert!(!source("mock/crates/c/src/tests/probe/cpu.slang").shipped());
    assert!(!source("light.slang").shipped());
}

#[test]
fn the_sources_of_a_repository_are_named_by_crate_and_skip_what_is_never_authored() {
    use std::collections::BTreeSet;
    use std::fs;
    let d = tempfile::tempdir().unwrap();
    for (rel, text) in [
        ("mock/crates/alpha/shaders/a.slang", "module a;\n"),
        ("mock/crates/alpha/src/tests/probe/p.slang", "import a;\n"),
        ("mock/target/x.slang", ""),
        ("mock/vendor/v/shaders/v.slang", ""),
        ("mock/research/n/r.slang", ""),
        ("mock/design_rounds/1/d.slang", ""),
    ] {
        let p = d.path().join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    let all: BTreeSet<String> = ["alpha".to_string()].into();
    let reading = mockspace_lint_rules::Reading::new(d.path());
    let found = sources(&reading.view(), d.path(), &d.path().join("mock"), &all);
    let got: Vec<_> = found.iter().map(|s| (s.rel.as_str(), s.crate_name.as_str())).collect();
    assert_eq!(
        got,
        [
            ("mock/crates/alpha/shaders/a.slang", "alpha"),
            ("mock/crates/alpha/src/tests/probe/p.slang", "alpha"),
        ]
    );
    assert_eq!(found[0].file.module.as_ref().map(|m| m.0.as_str()), Some("a"));
}
