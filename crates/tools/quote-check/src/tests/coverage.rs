//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

use super::Memory;
use crate::coverage::measure;
use crate::locator::Lines;

fn lines(a: usize, b: usize) -> Lines {
    Lines::new(a, b).unwrap()
}

fn source() -> Memory {
    Memory::default()
        .with("dev", "seed/A.md", "one\n\ntwo\nthree\n")
        .with("branch", "seed/A.md", "one\n\ntwo\nthree\n")
        .with("branch", "notes/B.md", "only\n")
        .with("branch", "notes/C.png", "binary\n")
        .with("branch", "other/D.md", "elsewhere\n")
}

#[test]
fn ranges_from_two_revisions_cover_one_path_together() {
    let cited = [
        ("dev".to_owned(), "seed/A.md".to_owned(), lines(1, 2)),
        ("branch".to_owned(), "seed/A.md".to_owned(), lines(3, 4)),
    ];
    let c = measure(&mut source(), &cited, "seed");
    assert_eq!(c.whole, 1);
    assert!(c.partial.is_empty());
}

#[test]
fn a_worded_line_no_range_covers_is_counted_and_a_blank_one_is_not() {
    let cited = [("dev".to_owned(), "seed/A.md".to_owned(), lines(1, 3))];
    let c = measure(&mut source(), &cited, "seed");
    assert_eq!(c.partial, [("seed/A.md".to_owned(), 1, 3)]);
}

#[test]
fn uncited_text_files_are_listed_under_the_prefix_and_others_are_not() {
    let cited = [("branch".to_owned(), "seed/A.md".to_owned(), lines(1, 4))];
    let all = measure(&mut source(), &cited, "");
    assert_eq!(all.uncited, [
        ("notes/B.md".to_owned(), 1),
        ("other/D.md".to_owned(), 1)
    ]);
    let within = measure(&mut source(), &cited, "notes");
    assert_eq!(within.uncited, [("notes/B.md".to_owned(), 1)]);
}
