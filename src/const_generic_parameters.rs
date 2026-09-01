//! The one position where a bare primitive is permitted: the type of a const
//! generic parameter.
//!
//! The bare-primitive lints scan lines rather than syntax, which is why they
//! reach a literal suffix and a cast that a signature walk would miss. The cost
//! is that a line has no idea where in the grammar its tokens sit, so
//! `<const BITS: u32>` and `const BITS: u32 = 32;` read the same to it. Only one
//! of the two is permitted, so the position has to come from the parse.
//!
//! What this does is narrow on purpose: parse the file, find the type of every
//! const generic parameter, and blank those bytes out before the line scan runs.
//! Everything else, the argument at a use site, an associated constant, an item
//! constant, a field, a cast, a suffix, is untouched and still reported.
//!
//! Blanking keeps the byte length and every newline, so the line numbers the
//! scan reports are the file's own and a caller can still read `lint:allow` off
//! the raw line.

use core::ops::Range;

use tree_sitter::{Node, Parser};

/// The source with the type of every const generic parameter blanked out.
///
/// Returns a copy the caller scans instead of the original. On a parse failure
/// the original comes back unchanged, so a file this cannot read stays fully
/// enforced rather than silently excepted.
#[must_use]
pub fn without_const_generic_parameter_types(source: &str) -> String {
    let spans = const_generic_parameter_type_spans(source);
    if spans.is_empty() {
        return source.to_string();
    }
    blank_spans(source, &spans)
}

/// The byte range of the type of every const generic parameter in the file.
///
/// Empty when the file has none, and empty when it does not parse.
#[must_use]
pub fn const_generic_parameter_type_spans(source: &str) -> Vec<Range<usize>> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };

    let mut spans = Vec::new();
    collect(tree.root_node(), &mut spans);
    spans
}

fn collect(node: Node, spans: &mut Vec<Range<usize>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "const_parameter" {
            if let Some(ty) = child.child_by_field_name("type") {
                spans.push(ty.byte_range());
            }
        }
        collect(child, spans);
    }
}

/// Replace every byte in `spans` with a space, keeping newlines where they are.
///
/// Byte length is preserved, so an offset outside the spans still names the same
/// character and the line a scan reports is the line the file has.
fn blank_spans(source: &str, spans: &[Range<usize>]) -> String {
    let mut bytes = source.as_bytes().to_vec();
    for span in spans {
        if span.end > bytes.len() {
            continue;
        }
        for byte in &mut bytes[span.start .. span.end] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    }
    // Every replaced byte was inside one node's range and every replacement is
    // ASCII, so the result is still the original's encoding.
    String::from_utf8(bytes).unwrap_or_else(|_| source.to_string())
}

#[cfg(test)]
mod tests {
    use super::{blank_spans, const_generic_parameter_type_spans, without_const_generic_parameter_types};

    fn masked(source: &str) -> String {
        without_const_generic_parameter_types(source)
    }

    #[test]
    fn a_parameter_declaration_loses_its_type() {
        let out = masked("pub struct Signed<const BITS: u32>;\n");
        assert!(!out.contains("u32"), "the parameter's type must be blanked: {out:?}");
        assert!(out.contains("BITS"), "the parameter's name must survive: {out:?}");
    }

    #[test]
    fn an_impl_header_loses_its_parameter_types() {
        let out = masked("impl<const BITS: u32, const FRAC: i32> Format for UFixed<BITS, FRAC> {}\n");
        assert!(!out.contains("u32"), "{out:?}");
        assert!(!out.contains("i32"), "{out:?}");
    }

    #[test]
    fn a_function_parameter_type_is_untouched() {
        let out = masked("fn f<const N: usize>(x: u32) -> i64 { 0 }\n");
        assert!(!out.contains("usize"), "the const parameter's type goes: {out:?}");
        assert!(out.contains("u32"), "an ordinary parameter stays: {out:?}");
        assert!(out.contains("i64"), "a return type stays: {out:?}");
    }

    #[test]
    fn an_associated_constant_is_untouched() {
        let out = masked("pub trait Format { const PHASE_NUM: i64; }\n");
        assert!(out.contains("i64"), "an associated const is not a parameter: {out:?}");
    }

    #[test]
    fn an_item_constant_is_untouched() {
        let out = masked("const WIDTH: u32 = 32;\n");
        assert!(out.contains("u32"), "an item const is not a parameter: {out:?}");
    }

    #[test]
    fn a_struct_field_is_untouched() {
        let out = masked("pub struct Handle(u32);\n");
        assert!(out.contains("u32"), "{out:?}");
    }

    #[test]
    fn a_use_site_argument_carries_no_type_to_blank() {
        let source = "type Byte = Signed<8>;\n";
        assert!(const_generic_parameter_type_spans(source).is_empty());
    }

    #[test]
    fn a_default_value_is_not_the_type() {
        let out = masked("pub struct Buf<const N: usize = 8>;\n");
        assert!(!out.contains("usize"), "{out:?}");
        assert!(out.contains('8'), "the default stays: {out:?}");
    }

    #[test]
    fn a_lifetime_or_type_parameter_is_not_a_const_parameter() {
        let source = "pub struct Pair<'a, T, const N: u8>(&'a T);\n";
        let spans = const_generic_parameter_type_spans(source);
        assert_eq!(spans.len(), 1, "only the const parameter has a type to blank");
        assert_eq!(&source[spans[0].clone()], "u8");
    }

    #[test]
    fn a_nested_parameter_inside_a_module_is_found() {
        let source = "mod points {\n    pub struct Integer<const BITS: u32>;\n}\n";
        let out = masked(source);
        assert!(!out.contains("u32"), "the walk has to descend: {out:?}");
    }

    #[test]
    fn a_where_clause_bound_is_untouched() {
        let source = "impl<const B: u32> T for S<B> where Signed<B>: Slots, u32: Copy {}\n";
        let out = masked(source);
        assert!(
            out.contains("u32: Copy"),
            "only the parameter's own type goes, not a bound naming the same type: {out:?}"
        );
    }

    #[test]
    fn a_syntax_error_cannot_blank_a_position_that_is_not_a_parameter() {
        // The direction that matters: a broken file must not except more than a
        // whole one would, or a planted syntax error becomes a way through.
        let source = "pub struct Broken(u64 ;\nfn f(x: u32) {}\n";
        let out = masked(source);
        assert!(out.contains("u64"), "a field stays reported: {out:?}");
        assert!(out.contains("u32"), "a parameter stays reported: {out:?}");
        assert_eq!(out.len(), source.len());
    }

    #[test]
    fn byte_length_and_line_count_survive() {
        let source = "pub struct A<const N: u32>;\npub struct B(u64);\n";
        let out = masked(source);
        assert_eq!(out.len(), source.len(), "offsets have to keep meaning");
        assert_eq!(out.lines().count(), source.lines().count());
        assert_eq!(
            out.lines().nth(1),
            Some("pub struct B(u64);"),
            "an untouched line comes back exactly: {out:?}"
        );
    }

    #[test]
    fn multi_byte_text_outside_a_span_survives() {
        let source = "/// kokoluokka ä\npub struct A<const N: u32>;\n";
        let out = masked(source);
        assert!(out.contains('ä'), "{out:?}");
        assert!(!out.contains("u32"), "{out:?}");
    }

    #[test]
    fn blanking_nothing_changes_nothing() {
        let source = "pub struct Handle(u32);\n";
        assert_eq!(blank_spans(source, &[]), source);
    }

    #[test]
    fn a_span_past_the_end_is_ignored_rather_than_panicking() {
        let source = "abc\n";
        assert_eq!(blank_spans(source, &[0 .. 99]), source);
    }
}
