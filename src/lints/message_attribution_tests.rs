//! The `message-attribution` lint, driven by hand on messages: the deny path, the
//! permit path, the keys that configure it, and what it takes to be an advert.
//!
//! What names an agent is held to the conformance table in `agent_identity`, and
//! the commit identity through the pack as the engine drives it is under `tests/`.

use mockspace_lint_rules::MessageDomain;

use super::*;

fn check(l: &MessageAttribution, mode: AgentMode, msg: &str) -> Vec<String> {
    let ctx = MessageContext::new(
        MessageDomain::CommitMessage,
        mode,
        msg,
        "COMMIT_EDITMSG",
        std::path::Path::new("/tmp"),
    );
    l.check_message(&ctx)
        .into_iter()
        .map(|e| e.finding_kind.unwrap_or("none").to_string())
        .collect()
}

fn with(pairs: &[(&str, &str)]) -> MessageAttribution {
    let mut l = MessageAttribution::default();
    let mut p = HashMap::new();
    for (k, v) in pairs {
        p.insert((*k).to_string(), (*v).to_string());
    }
    l.configure(&p);
    l
}

// --- the deny path, which is the default ---

#[test]
fn an_agent_byline_is_denied_when_a_human_was_in_the_loop() {
    let l = MessageAttribution::default();
    assert_eq!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nCo-Authored-By: Claude <noreply@anthropic.com>"
        ),
        vec!["byline"]
    );
}

#[test]
fn adverts_are_denied_in_both_modes() {
    let l = MessageAttribution::default();
    for mode in [AgentMode::Assistant, AgentMode::Autonomous] {
        assert_eq!(
            check(
                &l,
                mode,
                "feat: x\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)"
            ),
            vec!["advert"],
            "adverts must be denied under {mode:?}"
        );
    }
}

#[test]
fn a_session_trailer_is_an_advert() {
    let l = MessageAttribution::default();
    assert_eq!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nClaude-Session: https://claude.ai/code/session_abc"
        ),
        vec!["advert"]
    );
}

#[test]
fn a_human_co_author_is_never_touched() {
    // The rule is about agent provenance, so a real person's co-authorship
    // must pass in every mode and under every configuration.
    let l = MessageAttribution::default();
    for mode in [AgentMode::Assistant, AgentMode::Autonomous] {
        assert!(
            check(
                &l,
                mode,
                "feat: x\n\nCo-Authored-By: Jane Smith <jane@example.com>"
            )
            .is_empty()
        );
    }
}

#[test]
fn a_github_noreply_human_is_not_an_agent() {
    // `12345+user@users.noreply.github.com` is a real person hiding their
    // address, and matching on `noreply@` would have caught them.
    let l = MessageAttribution::default();
    assert!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nCo-Authored-By: Some One <12345+someone@users.noreply.github.com>"
        )
        .is_empty()
    );
}

// --- the PERMIT path, which the default config never exercises ---

#[test]
fn a_configured_byline_is_permitted_under_headless_mode() {
    // The permit path. Without a test here, a matcher that always denied
    // would pass every other test in this file.
    let l = with(&[("autonomous", "Claude *<noreply@anthropic.com>")]);
    assert!(
        check(
            &l,
            AgentMode::Autonomous,
            "feat: x\n\nCo-Authored-By: Claude Opus <noreply@anthropic.com>"
        )
        .is_empty(),
        "a byline matching the configured glob must be permitted"
    );
}

#[test]
fn the_same_byline_is_still_denied_with_a_human_in_the_loop() {
    // The two modes must genuinely differ, or the predicate is decoration.
    let l = with(&[("autonomous", "Claude *<noreply@anthropic.com>")]);
    assert_eq!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nCo-Authored-By: Claude Opus <noreply@anthropic.com>"
        ),
        vec!["byline"]
    );
}

#[test]
fn a_non_matching_byline_is_denied_even_under_headless_mode() {
    // A recognised agent identity that fails the configured glob. Headless
    // mode permits one specific byline, not any byline.
    let l = with(&[("autonomous", "Claude *<noreply@anthropic.com>")]);
    assert_eq!(
        check(
            &l,
            AgentMode::Autonomous,
            "feat: x\n\nCo-Authored-By: Copilot <x@github.test>"
        ),
        vec!["byline"]
    );
}

#[test]
fn an_unrecognised_bot_reads_as_a_human_until_the_list_names_it() {
    // Inherent to matching on identity: a bot nobody has listed is
    // indistinguishable from a person. This is why `agent_identities` is
    // configuration rather than a fixed rule, and the behaviour is asserted
    // so it is a known boundary rather than a surprise.
    let l = MessageAttribution::default();
    assert!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nCo-Authored-By: Somebot <x@evil.test>"
        )
        .is_empty()
    );
    let named = with(&[("agent_identities", "claude,somebot")]);
    assert_eq!(
        check(
            &named,
            AgentMode::Assistant,
            "feat: x\n\nCo-Authored-By: Somebot <x@evil.test>"
        ),
        vec!["byline"]
    );
}

#[test]
fn headless_mode_requires_the_byline_it_configures() {
    // Provenance is the point of the mode: work with nobody watching and no
    // byline has no record of who produced it.
    let l = with(&[("autonomous", "Claude *")]);
    assert_eq!(check(&l, AgentMode::Autonomous, "feat: x"), vec![
        "missing-byline"
    ]);
    // and it is not required when the mode does not configure one
    let bare = MessageAttribution::default();
    assert!(check(&bare, AgentMode::Autonomous, "feat: x").is_empty());
}

#[test]
fn a_permitted_advert_glob_lets_one_through() {
    // The advert permit path, so "deny by default" is proven to be a default
    // rather than a hardcoded rule.
    let l = with(&[("adverts", "*Generated with [OurTool]*")]);
    assert!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nGenerated with [OurTool](https://x.test)"
        )
        .is_empty()
    );
}

// --- configurability ---

#[test]
fn a_project_can_add_its_own_advert_pattern() {
    let l = with(&[("extra_patterns", "powered by robotron")]);
    assert_eq!(
        check(&l, AgentMode::Assistant, "feat: x\n\nPowered By Robotron"),
        vec!["advert"]
    );
    // and the shipped defaults still apply alongside it
    assert_eq!(check(&l, AgentMode::Assistant, "feat: x\n\n🤖"), vec![
        "advert"
    ]);
}

#[test]
fn a_project_can_replace_the_default_advert_set_entirely() {
    let l = with(&[("advert_patterns", "only-this")]);
    assert!(check(&l, AgentMode::Assistant, "feat: x\n\n🤖").is_empty());
    assert_eq!(
        check(&l, AgentMode::Assistant, "feat: x\n\nonly-this"),
        vec!["advert"]
    );
}

#[test]
fn a_project_can_redefine_what_counts_as_an_agent() {
    let l = with(&[("agent_identities", "robotron")]);
    // Claude is no longer an agent identity here, so it reads as a human
    assert!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nCo-Authored-By: Claude <a@b.test>"
        )
        .is_empty()
    );
    assert_eq!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nCo-Authored-By: Robotron <a@b.test>"
        ),
        vec!["byline"]
    );
}

#[test]
fn a_project_can_name_a_name_as_an_agent_whatever_the_mailbox() {
    let msg = "feat: x\n\nCo-Authored-By: claude <root@buildhost.local>";
    // the default lets a bare given name behind a mailbox of nobody's through
    assert!(check(&MessageAttribution::default(), AgentMode::Assistant, msg).is_empty());
    let l = with(&[("agent_names", "Claude")]);
    assert_eq!(check(&l, AgentMode::Assistant, msg), vec!["byline"]);
    // the whole name, so a person called Claude Monet is still a person
    assert!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nCo-Authored-By: Claude Monet <root@buildhost.local>"
        )
        .is_empty()
    );
}

// --- robustness ---

#[test]
fn commented_lines_and_the_diff_are_not_authored() {
    let l = MessageAttribution::default();
    let msg = "feat: x\n\n# Co-Authored-By: Claude <a@b.test>\n\
               # ------------------------ >8 ------------------------\n\
               +Co-Authored-By: Claude <a@b.test>";
    assert!(check(&l, AgentMode::Assistant, msg).is_empty());
}

#[test]
fn the_trailer_key_is_matched_case_insensitively() {
    let l = MessageAttribution::default();
    for form in ["Co-Authored-By:", "co-authored-by:", "CO-AUTHORED-BY:"] {
        assert_eq!(
            check(
                &l,
                AgentMode::Assistant,
                &format!("feat: x\n\n{form} Claude Opus <a@b.test>")
            ),
            vec!["byline"],
            "{form} should be recognised"
        );
    }
}

#[test]
fn an_advert_that_looks_like_a_byline_is_reported_once_as_an_advert() {
    // `Co-Authored-By: Claude Code` names a tool, not a person, so it is an
    // advert. Reporting it twice would be noise.
    let l = MessageAttribution::default();
    assert_eq!(
        check(
            &l,
            AgentMode::Assistant,
            "feat: x\n\nCo-Authored-By: Claude Code <noreply@anthropic.com>"
        ),
        vec!["advert"]
    );
}

#[test]
fn the_glob_supports_star_and_question_mark() {
    assert!(glob_matches("Claude *", "Claude Opus <x@y>"));
    assert!(glob_matches(
        "*<noreply@anthropic.com>",
        "Claude <noreply@anthropic.com>"
    ));
    assert!(glob_matches("a?c", "abc"));
    assert!(!glob_matches("a?c", "ac"));
    assert!(!glob_matches("Claude *", "Somebot <x@y>"));
    // an empty pattern permits nothing, so unconfigured means denied
    assert!(!glob_matches("", "anything"));
}

#[test]
fn every_finding_kind_the_lint_emits_is_declared() {
    let l = with(&[("autonomous", "Claude *")]);
    let declared = l.finding_kinds();
    let mut emitted = Vec::new();
    emitted.extend(check(
        &l,
        AgentMode::Assistant,
        "x\n\nCo-Authored-By: Claude <a@b>",
    ));
    emitted.extend(check(&l, AgentMode::Assistant, "x\n\n🤖"));
    emitted.extend(check(&l, AgentMode::Autonomous, "x"));
    for kind in emitted {
        assert!(
            declared.contains(&kind.as_str()),
            "`{kind}` is not declared"
        );
    }
}
