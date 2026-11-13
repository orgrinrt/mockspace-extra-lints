use super::citation_path;

#[test]
fn a_citation_resolves_under_its_own_root_first() {
    let there = |p: &str| p == "research/seed/mediation.md";
    assert_eq!(
        citation_path("seed::mediation::#the-shape", there),
        Some("research/seed/mediation.md".to_string())
    );
}

#[test]
fn a_panel_citation_resolves_under_research_itself() {
    // The panel root is `research/`, so its citations carry the panel
    // directory as their first segment and there is no `research/panel/`.
    let there = |p: &str| p == "research/202608112000_the-shared-base-panel/INTENTS.md";
    assert_eq!(
        citation_path(
            "panel::202608112000_the-shared-base-panel::INTENTS::#k69",
            there
        ),
        Some("research/202608112000_the-shared-base-panel/INTENTS.md".to_string())
    );
}

#[test]
fn a_deep_citation_keeps_every_segment() {
    let there = |p: &str| p == "research/_prefork_seed/design_rounds/taxonomy.md";
    assert_eq!(
        citation_path("panel::_prefork_seed::design_rounds::taxonomy::104", there),
        Some("research/_prefork_seed/design_rounds/taxonomy.md".to_string())
    );
}

#[test]
fn a_citation_naming_nothing_resolves_to_nothing() {
    // The control. Without it a resolver returning a path unconditionally
    // would mark every document cited and the whole finding would invert:
    // the report would say the registry has absorbed everything.
    assert_eq!(citation_path("seed::nowhere::#a", |_| false), None);
}

#[test]
fn a_string_that_is_not_a_citation_is_left_alone() {
    assert_eq!(citation_path("just some prose", |_| true), None);
    assert_eq!(citation_path("seed::only-two", |_| true), None);
    assert_eq!(citation_path("", |_| true), None);
}
