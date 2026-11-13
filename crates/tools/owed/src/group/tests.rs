//! The reading of what a marker says: its class, its words, and which markers are one gap.

use std::collections::BTreeSet;

use super::*;

fn item(rel: &str, class: Class, reason: &str) -> Item {
    Item {
        rel: rel.to_string(),
        class,
        reason: reason.to_string(),
        area: area(rel),
        line: 1,
    }
}

fn shapes(items: &[Item]) -> Vec<(How, Vec<usize>)> {
    group(items).into_iter().map(|g| (g.how, g.items)).collect()
}

#[test]
fn a_tag_is_a_class_and_the_questions_are_the_canon_silence_and_op() {
    assert_eq!(Class::of(None), Class::Unmarked);
    assert_eq!(Class::of(Some("  ")), Class::Unmarked);
    assert_eq!(Class::of(Some("stand-in")), Class::StandIn);
    assert_eq!(Class::of(Some("Stand In")), Class::StandIn);
    assert_eq!(Class::of(Some("canon silent")), Class::CanonSilent);
    assert_eq!(Class::of(Some("canon-silent")), Class::CanonSilent);
    assert_eq!(Class::of(Some("op")), Class::Op);
    assert_eq!(
        Class::of(Some("gpu-pass")),
        Class::Seam("gpu-pass".to_string())
    );
    let owing: Vec<bool> = ["canon silent", "op", "stand-in", "gpu-pass"]
        .iter()
        .map(|t| Class::of(Some(t)).owes_a_decision())
        .collect();
    assert_eq!(owing, [true, true, false, false]);
}

#[test]
fn content_words_leave_out_short_words_and_the_ones_said_of_everything() {
    let w = words("The wind is pushed over the Branches, and 3 of them");
    assert_eq!(
        w,
        ["branches", "pushed", "them", "wind"]
            .map(String::from)
            .into()
    );
}

#[test]
fn the_overlap_is_what_is_shared_over_what_either_has() {
    let a: BTreeSet<String> = ["wind", "branch", "frame", "push"].map(String::from).into();
    let b: BTreeSet<String> = ["wind", "branch", "frame", "roof"].map(String::from).into();
    // three shared of five held
    assert!((jaccard(&a, &b) - 0.6).abs() < 1e-9);
    assert_eq!(jaccard(&a, &BTreeSet::new()), 0.0);
}

#[test]
fn the_threshold_is_where_two_markers_become_one_gap() {
    // the edge is exercised through the grouping itself
    let a = item(
        "mock/crates/kaski-a/src/lib.rs",
        Class::Unmarked,
        "alpha bravo charlie delta echo",
    );
    let near = item(
        "mock/crates/kaski-a/src/b.rs",
        Class::Unmarked,
        "alpha bravo charlie delta foxtrot",
    );
    let far = item(
        "mock/crates/kaski-a/src/c.rs",
        Class::Unmarked,
        "alpha bravo golf hotel india",
    );
    // a and near: 4 shared of 6 held; a and far: 2 shared of 8 held
    let g = shapes(&[a, near, far]);
    assert_eq!(g[0], (How::Similar, vec![0, 1]));
    assert_eq!(g[1], (How::Rest, vec![2]));
}

#[test]
fn a_chain_of_near_matches_does_not_pull_unlike_gaps_into_one() {
    let items = [
        item(
            "mock/crates/kaski-a/src/1.rs",
            Class::Unmarked,
            "alpha bravo charlie delta",
        ),
        item(
            "mock/crates/kaski-a/src/2.rs",
            Class::Unmarked,
            "alpha bravo charlie echo",
        ),
        item(
            "mock/crates/kaski-a/src/3.rs",
            Class::Unmarked,
            "bravo charlie echo foxtrot",
        ),
        item(
            "mock/crates/kaski-a/src/4.rs",
            Class::Unmarked,
            "charlie echo foxtrot golf",
        ),
    ];
    // 1~2 (3/5), 2~3 (3/5), 3~4 (3/5), but 1 and 3 share 2 of 6 and 1 and 4 share 1 of 7: the cluster
    // is led by the first marker, so 3 and 4 do not join it through 2
    let g = shapes(&items);
    assert_eq!(g[0], (How::Similar, vec![0, 1]));
    assert_eq!(g[1], (How::Similar, vec![2, 3]));
}

#[test]
fn a_reason_of_too_few_words_shares_only_with_the_same_words_in_its_crate() {
    let items = [
        item("mock/crates/kaski-a/src/1.rs", Class::StandIn, "later"),
        item("mock/crates/kaski-a/src/2.rs", Class::StandIn, "Later"),
        item("mock/crates/kaski-b/src/3.rs", Class::StandIn, "later"),
    ];
    let g = shapes(&items);
    // the two in kaski-a are one stock ticket; the one in kaski-b stands alone among the rest
    assert_eq!(g[0], (How::Stock, vec![0, 1]));
    assert_eq!(g[1], (How::Rest, vec![2]));
}

#[test]
fn the_rest_of_a_crate_of_one_kind_share_a_ticket_and_another_crate_or_kind_does_not() {
    let items = [
        item(
            "mock/crates/kaski-a/src/1.rs",
            Class::StandIn,
            "alpha bravo charlie",
        ),
        item(
            "mock/crates/kaski-a/src/2.rs",
            Class::StandIn,
            "delta echo foxtrot",
        ),
        item(
            "mock/crates/kaski-b/src/3.rs",
            Class::StandIn,
            "golf hotel india",
        ),
        item(
            "mock/crates/kaski-a/src/4.rs",
            Class::Unmarked,
            "juliet kilo lima",
        ),
    ];
    let g = group(&items);
    let rest: Vec<(&Class, &Vec<usize>)> = g.iter().map(|g| (&g.class, &g.items)).collect();
    assert!(rest.contains(&(&Class::StandIn, &vec![0, 1])), "{rest:?}");
    assert!(rest.contains(&(&Class::StandIn, &vec![2])), "{rest:?}");
    assert!(rest.contains(&(&Class::Unmarked, &vec![3])), "{rest:?}");
}

#[test]
fn every_marker_of_a_seam_tag_is_in_the_one_group_of_the_tag() {
    let items = [
        item(
            "mock/crates/kaski-a/src/1.rs",
            Class::Seam("gpu".into()),
            "alpha",
        ),
        item(
            "mock/crates/kaski-b/src/2.rs",
            Class::Seam("gpu".into()),
            "completely unlike words here",
        ),
        item(
            "mock/crates/kaski-b/src/3.rs",
            Class::Seam("net".into()),
            "alpha",
        ),
    ];
    let g = group(&items);
    assert_eq!(g.len(), 2);
    assert!(g.iter().all(|g| g.how == How::Seam));
    assert_eq!(g.iter().map(|g| g.items.len()).sum::<usize>(), 3);
}

#[test]
fn the_same_items_group_the_same_way_every_time() {
    let items: Vec<Item> = (0 .. 12)
        .map(|i| {
            item(
                &format!("mock/crates/kaski-{}/src/{i}.rs", i % 3),
                if i % 4 == 0 { Class::CanonSilent } else { Class::StandIn },
                &format!("gap number {} of the {} kind", i % 5, i % 2),
            )
        })
        .collect();
    let first = shapes(&items);
    for _ in 0 .. 5 {
        assert_eq!(shapes(&items), first);
    }
}

#[test]
fn the_area_of_a_path_is_its_crate_its_content_pack_or_its_tooling() {
    assert_eq!(area("mock/crates/kaski-sim/src/lib.rs"), "kaski-sim");
    assert_eq!(area("mock/crates/kaski-sim/DESIGN.md.tmpl"), "kaski-sim");
    assert_eq!(area("content/kaski/trees.toml"), "content/kaski");
    assert_eq!(area("mock/tools/owed/src/lib.rs"), "tool owed");
    assert_eq!(area("mock/lints/x.rs"), "lints");
    assert_eq!(area("mock/WORKFLOW.md.tmpl"), "mock design");
    assert_eq!(area("mock/agent/rules/a.md.tmpl"), "agent text");
    assert_eq!(area("README.md"), "root");
}

#[test]
fn a_subject_is_named_only_where_the_registry_has_a_row_for_it() {
    let subjects: BTreeSet<String> = ["weather", "workflow", "content"].map(String::from).into();
    assert_eq!(
        subject_of("kaski-weather", &subjects).as_deref(),
        Some("weather")
    );
    assert_eq!(subject_of("kaski-sim", &subjects), None);
    assert_eq!(
        subject_of("content/kaski", &subjects).as_deref(),
        Some("content")
    );
    assert_eq!(
        subject_of("tool owed", &subjects).as_deref(),
        Some("workflow")
    );
    assert_eq!(subject_of("mock design", &BTreeSet::new()), None);
}

#[test]
fn the_listed_words_of_a_reason_are_the_same_when_cut_again() {
    let long = "one two three four five six seven eight nine ten eleven twelve";
    assert_eq!(
        gist(long),
        "one two three four five six seven eight nine ten"
    );
    assert_eq!(gist(&gist(long)), gist(long));
    let wide = format!("{} {}", "x".repeat(90), "y".repeat(30));
    assert_eq!(gist(&wide), "x".repeat(90));
    assert_eq!(gist(&gist(&wide)), gist(&wide));
    assert_eq!(gist(""), "(no reason given)");
    assert_eq!(key("A Flat  ROOF"), "a flat roof");
}
