//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

use crate::locator::{Lines, Unreadable, parse};

#[test]
fn a_bare_file_name_sits_under_the_base() {
    let c = parse("DESIGN.md:47-58", "mock/research/seed").unwrap();
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].path, "mock/research/seed/DESIGN.md");
    assert_eq!(c[0].lines, Lines::new(47, 58).unwrap());
    assert_eq!(c[0].branch, None);
}

#[test]
fn a_path_with_a_directory_is_from_the_root_and_a_branch_is_kept_apart() {
    let c = parse(
        "docs/crate-taxonomy-panel:mock/registry/law/seam.toml:40-47",
        "mock/research/seed",
    )
    .unwrap();
    assert_eq!(c[0].path, "mock/registry/law/seam.toml");
    assert_eq!(c[0].branch.as_deref(), Some("docs/crate-taxonomy-panel"));
}

#[test]
fn a_file_at_the_root_on_a_branch_is_from_the_root_and_not_under_the_base() {
    let c = parse(
        "docs/crate-taxonomy-panel:mockspace.toml:411-462",
        "mock/research/seed",
    )
    .unwrap();
    assert_eq!(c[0].path, "mockspace.toml");
    assert_eq!(c[0].branch.as_deref(), Some("docs/crate-taxonomy-panel"));
}

#[test]
fn several_ranges_are_read_in_order() {
    let c = parse("DESIGN.md:3901-3903, DESIGN.md:3906-3910", "seed").unwrap();
    assert_eq!(
        c.iter()
            .map(|c| (c.lines.first(), c.lines.last()))
            .collect::<Vec<_>>(),
        [(3901, 3903), (3906, 3910)]
    );
}

#[test]
fn a_locator_that_cannot_be_read_is_refused_by_why() {
    assert_eq!(parse("  ", "seed"), Err(Unreadable::Empty));
    for bad in [
        "DESIGN.md",
        "DESIGN.md:12",
        "DESIGN.md:9-3",
        "DESIGN.md:0-3",
        "DESIGN.md:a-3",
        ":1-2",
    ] {
        assert!(
            matches!(parse(bad, "seed"), Err(Unreadable::Range(_))),
            "{bad} is refused"
        );
    }
}

#[test]
fn a_range_holds_its_ends_and_nothing_outside() {
    let r = Lines::new(3, 5).unwrap();
    assert!(r.holds(3) && r.holds(5));
    assert!(!r.holds(2) && !r.holds(6));
    assert_eq!(Lines::new(0, 1), None);
}

#[test]
fn a_bare_name_is_one_with_no_directory_and_no_branch() {
    let cited = |doc: &str| crate::locator::parse(doc, "").unwrap().remove(0);
    assert!(cited("seed-renderer.md:1-3").is_bare());
    assert!(
        !cited("mock/research/seed/seed-renderer.md:1-3").is_bare(),
        "a path with a directory"
    );
    assert!(
        !cited("dev:seed-renderer.md:1-3").is_bare(),
        "a branch-led path"
    );
    assert!(
        !crate::locator::parse("seed-renderer.md:1-3", "mock/research/seed").unwrap()[0].is_bare(),
        "the base applied"
    );
}
