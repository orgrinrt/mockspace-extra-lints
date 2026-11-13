//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

use crate::Options;

#[test]
fn the_two_positionals_and_the_flags_are_read() {
    let o = Options::read(&[
        "ikiuni-renderer",
        "../ikiuni-renderer",
        "--base",
        "mock/research/seed",
        "--coverage",
    ])
    .unwrap();
    assert_eq!(
        (o.namespace, o.source, o.base, o.coverage),
        (
            "ikiuni-renderer",
            "../ikiuni-renderer",
            "mock/research/seed",
            true
        )
    );
    assert_eq!(
        (o.quote, o.doc, o.rev, o.separator),
        ("quote", "doc", "rev", "---")
    );
}

#[test]
fn a_flag_it_does_not_know_and_a_missing_source_are_refused() {
    assert!(Options::read(&["ns", "src", "--bogus"]).is_err());
    assert!(Options::read(&["ns"]).is_err());
    assert!(Options::read(&["ns", "src", "--base"]).is_err());
}

#[test]
fn a_run_without_a_base_whose_bare_names_went_missing_is_refused_with_the_flag_named() {
    let why = crate::unbased("", 3451).expect("refused");
    assert!(
        why.contains("3451 rows") && why.contains("--base <dir>"),
        "{why}"
    );
    assert_eq!(
        crate::unbased("mock/research/seed", 3451),
        None,
        "a given base is not second-guessed"
    );
    assert_eq!(
        crate::unbased("", 0),
        None,
        "nothing missing, nothing refused"
    );
}
