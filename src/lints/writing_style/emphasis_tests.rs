use super::{emphasis_runs, strip_code_spans};

fn markers(prose: &str) -> Vec<&'static str> {
    emphasis_runs(prose).into_iter().map(|(_, m)| m).collect()
}

#[test]
fn bold_is_caught_in_both_spellings() {
    assert_eq!(markers("a **loud** claim"), vec!["**"]);
    assert_eq!(markers("a __loud__ claim"), vec!["__"]);
}

#[test]
fn italic_is_caught_in_both_spellings() {
    assert_eq!(markers("a *quiet* claim"), vec!["*"]);
    assert_eq!(markers("a _quiet_ claim"), vec!["_"]);
}

#[test]
fn a_line_with_several_runs_reports_once() {
    assert_eq!(markers("**one** and **two** and **three**"), vec!["**"]);
}

#[test]
fn the_line_number_is_the_line_it_sits_on() {
    let prose = "clean\nclean\na **loud** claim\nclean";
    assert_eq!(emphasis_runs(prose), vec![(3, "**")]);
}

#[test]
fn every_line_carrying_emphasis_is_reported() {
    let prose = "**one**\nclean\n*two*";
    assert_eq!(emphasis_runs(prose), vec![(1, "**"), (3, "*")]);
}

#[test]
fn bold_is_reported_as_bold_rather_than_twice_as_italic() {
    // The doubled form contains the single one, so a naive check would
    // report `**bold**` under both markers.
    assert_eq!(markers("**bold**"), vec!["**"]);
}

// The negatives. Each of these is a construction the ban does not reach, and
// each would fire under a plainer `contains('*')` check.

#[test]
fn plain_prose_is_left_alone() {
    assert!(markers("a claim carrying its weight in the words").is_empty());
}

#[test]
fn multiplication_is_not_emphasis() {
    assert!(markers("the area is a * b, in whole pixels").is_empty());
    assert!(markers("width * height * 4").is_empty());
}

#[test]
fn a_snake_case_identifier_is_not_emphasis() {
    assert!(markers("takes a from_raw and a to_raw").is_empty());
    assert!(markers("one_two_three_four").is_empty());
}

#[test]
fn a_leading_underscore_is_not_emphasis() {
    assert!(markers("_unused and _opaque are both fine").is_empty());
}

#[test]
fn a_markdown_list_bullet_is_not_emphasis() {
    assert!(markers("* one item").is_empty());
    assert!(markers("* one\n* two\n* three").is_empty());
}

#[test]
fn a_horizontal_rule_is_not_emphasis() {
    assert!(markers("---").is_empty());
    assert!(markers("***").is_empty());
}

#[test]
fn a_pointer_type_is_not_emphasis() {
    assert!(markers("takes a *mut Proxy and returns a *const Interface").is_empty());
}

#[test]
fn a_doc_comment_prefix_does_not_confuse_it() {
    assert_eq!(markers("/// A **loud** claim."), vec!["**"]);
    assert!(markers("/// A plain claim.").is_empty());
}

#[test]
fn emphasis_inside_a_code_span_does_not_reach_the_check() {
    // The pipeline strips code spans before this runs, so the guarantee is
    // about the pair rather than about `emphasis_runs` alone.
    let stripped = strip_code_spans("the literal `**` is not emphasis");
    assert!(markers(&stripped).is_empty());
}

#[test]
fn emphasis_inside_a_fenced_block_does_not_reach_the_check() {
    let stripped = strip_code_spans("prose\n```\nlet x = **y;\n```\nmore prose");
    assert!(markers(&stripped).is_empty());
}

#[test]
fn an_unclosed_marker_is_not_emphasis() {
    assert!(markers("a *start with no end").is_empty());
    assert!(markers("a **start with no end").is_empty());
}

#[test]
fn an_empty_line_is_not_emphasis() {
    assert!(markers("").is_empty());
    assert!(markers("\n\n\n").is_empty());
}

// The shapes plain `//` comments bring in, which the doc-comment-only scan
// never had to survive. Each is real Rust and none of it is emphasis.

#[test]
fn a_double_deref_pair_is_not_emphasis() {
    assert!(markers("let c = **a + **b;").is_empty());
    assert!(markers("**a + **b").is_empty());
    assert!(markers("compare **left with **right").is_empty());
}

#[test]
fn a_double_deref_that_closes_a_word_is_still_not_emphasis() {
    // The closing side needs a word before it and a non-word after, which a
    // deref never has, because the star leads its operand.
    assert!(markers("takes **x and **y and returns z").is_empty());
}

#[test]
fn a_double_reference_type_is_not_emphasis() {
    assert!(markers("fn f(a: **const T, b: **mut U)").is_empty());
}

#[test]
fn a_run_longer_than_the_marker_is_not_emphasis() {
    assert!(markers("***").is_empty());
    assert!(markers("___").is_empty());
    assert!(markers("****").is_empty());
    assert!(markers("a ***b*** c").is_empty());
}

#[test]
fn real_bold_still_fires_on_a_line_that_also_holds_a_deref() {
    assert_eq!(markers("**loud** beside **a"), vec!["**"]);
}
