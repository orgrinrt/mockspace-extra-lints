//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

use super::Memory;
use crate::compare::{Verdict, compare};
use crate::locator::parse;

const DOC: &str =
    "# Title\n\nThe first line of prose.\nThe second line of prose.  \n\n## Next\nA third line.\n";

fn check(doc: &str, quote: &str) -> Verdict {
    let mut src = Memory::default().with("r1", "seed/DOC.md", DOC);
    let cited = parse(doc, "seed").expect("the locator reads");
    compare(&mut src, "r1", &cited, quote, "---")
}

#[test]
fn a_quote_that_is_the_lines_is_exact() {
    assert_eq!(
        check(
            "DOC.md:3-4",
            "The first line of prose.\nThe second line of prose.  \n"
        ),
        Verdict::Exact
    );
}

#[test]
fn space_dropped_at_a_line_end_is_trailing_and_never_exact() {
    assert_eq!(
        check(
            "DOC.md:3-4",
            "The first line of prose.\nThe second line of prose.\n"
        ),
        Verdict::Trailing
    );
}

#[test]
fn a_carriage_return_reads_as_trailing_space() {
    assert_eq!(
        check(
            "DOC.md:3-4",
            "The first line of prose.\r\nThe second line of prose.\r\n"
        ),
        Verdict::Trailing
    );
}

#[test]
fn a_blank_line_closing_the_range_dropped_is_edge_blank_and_never_trailing() {
    assert_eq!(
        check(
            "DOC.md:3-5",
            "The first line of prose.\nThe second line of prose.\n"
        ),
        Verdict::EdgeBlank
    );
}

#[test]
fn two_ranges_joined_by_the_separator_are_joined() {
    let quote = "The first line of prose.\n---\nA third line.\n";
    assert_eq!(check("DOC.md:3-3, DOC.md:7-7", quote), Verdict::Joined);
}

#[test]
fn a_separator_inside_one_range_is_a_difference_and_never_joined() {
    let quote = "The first line of prose.\n---\nThe second line of prose.\n";
    assert!(matches!(check("DOC.md:3-4", quote), Verdict::Differs {
        at: 2,
        ..
    }));
}

#[test]
fn tabs_and_doubled_spaces_inside_a_line_are_spacing() {
    assert_eq!(
        check("DOC.md:3-3", "The\tfirst  line of prose.\n"),
        Verdict::Spacing
    );
}

#[test]
fn a_changed_word_differs_at_its_line_and_names_both_sides() {
    let v = check(
        "DOC.md:3-4",
        "The first line of prose.\nThe second line of verse.\n",
    );
    assert_eq!(v, Verdict::Differs {
        at:     2,
        cited:  "The second line of prose.".to_owned(),
        quoted: "The second line of verse.".to_owned(),
    });
    assert!(!v.faithful());
}

#[test]
fn a_quote_short_of_its_range_differs_where_it_stops() {
    assert!(matches!(
        check("DOC.md:3-4", "The first line of prose.\n"),
        Verdict::Differs {
            at: 2,
            ..
        }
    ));
}

#[test]
fn a_file_the_revision_lacks_is_no_file() {
    assert_eq!(
        check("GONE.md:1-1", "x\n"),
        Verdict::NoFile("seed/GONE.md".to_owned())
    );
}

#[test]
fn a_range_past_the_end_is_past_end_and_the_line_after_the_last_is_not() {
    assert!(matches!(
        check("DOC.md:7-9", "A third line.\n"),
        Verdict::PastEnd {
            last: 9,
            lines: 7,
            ..
        }
    ));
    assert_eq!(check("DOC.md:7-8", "A third line.\n"), Verdict::Exact);
}

#[test]
fn the_faithful_verdicts_are_the_five_quirks_and_exact() {
    for v in [
        Verdict::Exact,
        Verdict::Trailing,
        Verdict::EdgeBlank,
        Verdict::Joined,
        Verdict::Spacing,
    ] {
        assert!(v.faithful(), "{} is faithful", v.name());
    }
    assert!(!Verdict::NoFile(String::new()).faithful());
    assert!(
        !Verdict::PastEnd {
            path:  String::new(),
            last:  2,
            lines: 1,
        }
        .faithful()
    );
}
