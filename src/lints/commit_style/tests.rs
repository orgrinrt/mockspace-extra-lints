use super::*;

fn hiisi() -> CommitStyle {
    let mut l = CommitStyle::default();
    let mut p = HashMap::new();
    p.insert("types".into(), "feat,fix,refactor,docs,test,chore".into());
    p.insert("require_type".into(), "true".into());
    p.insert("max_subject".into(), "72".into());
    p.insert("subject_case".into(), "lower".into());
    p.insert("allow_scope".into(), "false".into());
    p.insert("forbid_trailing_period".into(), "true".into());
    p.insert("require_blank_line".into(), "true".into());
    l.configure(&p);
    l
}

fn conventional() -> CommitStyle {
    let mut l = CommitStyle::default();
    let mut p = HashMap::new();
    p.insert(
        "types".into(),
        "feat,fix,docs,style,refactor,test,chore".into(),
    );
    p.insert("require_type".into(), "true".into());
    p.insert("allow_scope".into(), "true".into());
    l.configure(&p);
    l
}

fn check(l: &CommitStyle, msg: &str) -> Vec<String> {
    let ctx = MessageContext::new(
        MessageDomain::CommitMessage,
        mockspace_lint_rules::AgentMode::Assistant,
        msg,
        "COMMIT_EDITMSG",
        std::path::Path::new("/tmp"),
    );
    l.check_message(&ctx)
        .into_iter()
        .map(|e| e.finding_kind.unwrap_or("none").to_string())
        .collect()
}

/// What an agent's PreToolUse hook actually hands over: the serialised tool
/// input as the text, and the command beside it as the invocation.
fn check_from_a_hook(l: &CommitStyle, command: &str) -> Vec<String> {
    let serialised = format!(
        "{{\"command\":\"{}\",\"description\":\"commit the change\"}}",
        command.replace('"', "\\\"")
    );
    let ctx = MessageContext::new(
        MessageDomain::CommitMessage,
        mockspace_lint_rules::AgentMode::Assistant,
        &serialised,
        "<stdin>",
        std::path::Path::new("/tmp"),
    )
    .with_invocation(Some(mockspace_lint_rules::Invocation {
        command:   Some(command),
        tool_name: Some("Bash"),
    }));
    l.check_message(&ctx)
        .into_iter()
        .map(|e| e.finding_kind.unwrap_or("none").to_string())
        .collect()
}

#[test]
fn a_good_subject_through_a_hook_is_not_a_finding() {
    // The defect this pair exists for. The text handed over is JSON, whose
    // first line begins `{"command":"git commit -m 'fix`, so reading it as
    // the subject reported a bad type and a length that was the length of
    // the command. Every commit from an agent's shell was refused, and the
    // message it was refused for was never the message.
    let l = hiisi();
    assert_eq!(
        check_from_a_hook(&l, "git commit -m 'fix: a subject well under the limit'"),
        Vec::<String>::new()
    );
}

#[test]
fn a_bad_subject_through_a_hook_is_still_caught() {
    // The half that keeps the arm above from being a hole: if reading the
    // command meant reading nothing, every commit would pass instead of
    // every commit failing, which is worse and quieter.
    let l = hiisi();
    let found = check_from_a_hook(
        &l,
        "git commit -m 'Added Some Things And This Subject Runs Well Past The Seventy Two Character Limit.'",
    );
    assert!(
        found.contains(&"subject-length".to_string()),
        "the length went unreported: {found:?}"
    );
    assert!(
        found.contains(&"subject-case".to_string()),
        "the case went unreported: {found:?}"
    );
}

#[test]
fn a_hook_call_that_authors_no_message_reports_nothing() {
    // An editor commit, or one whose message is in a file. The subject is
    // somewhere this cannot read, and the `commit-msg` hook is handed the
    // real file moments later, so declining costs no coverage and guessing
    // would refuse a message nobody has seen yet.
    let l = hiisi();
    assert_eq!(check_from_a_hook(&l, "git commit"), Vec::<String>::new());
    assert_eq!(
        check_from_a_hook(&l, "git commit -F /tmp/msg.txt"),
        Vec::<String>::new()
    );
}

#[test]
fn an_invocation_carrying_no_command_does_not_put_the_tool_input_in_front_of_the_subject() {
    // What an MCP commit tool looks like here. The gate spells
    // `--command "$COMMAND"` unconditionally and such a tool has no shell
    // command, so `Some("")` arrives, and falling back to `ctx.message`
    // would put the serialised input back where the subject belongs, which
    // is the defect this whole change removes.
    //
    // The FIXME on `subject_source` is the other half: this leaves such a
    // tool unchecked, and closing that is the gate's job rather than this
    // lint's.
    let l = hiisi();
    let serialised = "{\"message\":\"Added Some Things.\",\"repo\":\"muisti\"}";
    let ctx = MessageContext::new(
        MessageDomain::CommitMessage,
        mockspace_lint_rules::AgentMode::Assistant,
        serialised,
        "<stdin>",
        std::path::Path::new("/tmp"),
    )
    .with_invocation(Some(mockspace_lint_rules::Invocation {
        command:   Some(""),
        tool_name: Some("mcp__git__commit"),
    }));
    let found: Vec<String> = l
        .check_message(&ctx)
        .into_iter()
        .map(|e| e.finding_kind.unwrap_or("none").to_string())
        .collect();
    assert!(
        found.is_empty(),
        "the serialised input was judged as a subject: {found:?}"
    );
}

#[test]
fn the_commit_msg_hook_path_is_untouched() {
    // The control for the three above: with no invocation the text is the
    // message, which is how the git hook calls it, and every existing arm
    // in this file goes through that path.
    let l = hiisi();
    assert!(check(&l, "fix: a subject well under the limit").is_empty());
    assert!(!check(&l, "Added Some Thing.").is_empty());
}

#[test]
fn an_unconfigured_lint_imposes_nothing() {
    // The load-bearing default: declaring no convention means no findings,
    // because no commit convention fits every project.
    let l = CommitStyle::default();
    assert!(check(&l, "Added Some Thing.").is_empty());
    assert!(check(&l, "whatever").is_empty());
}

#[test]
fn the_hiisi_preset_accepts_its_own_form() {
    let l = hiisi();
    assert!(check(&l, "feat: add catalogue entry validation").is_empty());
    assert!(check(&l, "fix: handle missing loadout").is_empty());
    assert!(check(&l, "feat!: change the language trait signature").is_empty());
    assert!(check(&l, "docs: update the data store design\n\nbody here").is_empty());
}

#[test]
fn the_hiisi_preset_rejects_each_thing_it_forbids() {
    let l = hiisi();
    assert_eq!(check(&l, "feat(data): add thing"), vec!["scope-forbidden"]);
    assert_eq!(check(&l, "feat: Add Data Store"), vec!["subject-case"]);
    assert_eq!(check(&l, "feat: add data store."), vec!["trailing-period"]);
    // both, and correctly so: it has no type prefix and it is capitalised.
    // With no type to strip, the case rule applies to the whole subject.
    assert_eq!(check(&l, "Added new feature"), vec![
        "missing-type",
        "subject-case"
    ]);
    assert_eq!(check(&l, "wip: something"), vec!["unknown-type"]);
    assert_eq!(check(&l, "feat: x\nbody with no blank line"), vec![
        "missing-blank-line"
    ]);
}

#[test]
fn conventional_permits_the_scope_that_hiisi_forbids() {
    // The two presets must genuinely differ, or shipping both is pointless.
    let c = conventional();
    assert!(check(&c, "feat(parser): add lookahead").is_empty());
    assert_eq!(check(&hiisi(), "feat(parser): add lookahead"), vec![
        "scope-forbidden"
    ]);
}

#[test]
fn conventional_leaves_case_and_period_alone() {
    // It constrains the type vocabulary and nothing else, so a project
    // adopting it is not silently also adopting the stricter rules.
    let c = conventional();
    assert!(check(&c, "feat: Add A Thing.").is_empty());
}

#[test]
fn the_subject_length_limit_counts_characters_not_bytes() {
    let mut l = CommitStyle::default();
    let mut p = HashMap::new();
    p.insert("max_subject".into(), "10".into());
    l.configure(&p);
    // ten multi-byte characters is ten characters, not thirty bytes
    assert!(check(&l, "ääääääääää").is_empty());
    assert_eq!(check(&l, "ãããããããããää"), vec!["subject-length"]);
}

#[test]
fn git_generated_subjects_are_left_alone() {
    // The user did not author these, so holding them to an authored
    // convention would block commits nobody wrote the subject for.
    let l = hiisi();
    for msg in [
        "Merge branch 'dev' into feat/x",
        "Revert \"feat: add thing\"",
        "fixup! feat: add thing",
        "squash! feat: add thing",
    ] {
        assert!(check(&l, msg).is_empty(), "{msg} should be exempt");
    }
}

#[test]
fn commented_lines_and_the_diff_are_not_the_subject() {
    // git's template puts help text and, under --verbose, a whole diff into
    // the file. Neither is authored, so neither can be a violation.
    let l = hiisi();
    let msg = "feat: add thing\n\n# Please enter the commit message...\n# feat(bad): Example.\n\
               # ------------------------ >8 ------------------------\ndiff --git a/x b/x\n+Added Thing.";
    assert!(check(&l, msg).is_empty());
}

#[test]
fn an_empty_subject_is_reported_once() {
    let l = hiisi();
    assert_eq!(check(&l, ""), Vec::<String>::new());
    assert_eq!(check(&l, "   \n\nbody"), vec!["empty-subject"]);
}

#[test]
fn a_breaking_marker_does_not_make_the_type_unknown() {
    let l = hiisi();
    assert!(check(&l, "refactor!: change the trait").is_empty());
}

#[test]
fn an_unclosed_scope_parenthesis_is_reported() {
    let c = conventional();
    assert_eq!(check(&c, "feat(parser: add lookahead"), vec![
        "unknown-type"
    ]);
}

#[test]
fn every_finding_kind_the_lint_emits_is_declared() {
    // Undeclared kinds cannot be given a per-finding severity in config, so
    // the emitted set and the declared set must agree.
    let l = hiisi();
    let declared = l.finding_kinds();
    let mut emitted: Vec<String> = Vec::new();
    for msg in [
        "feat(data): add thing",
        "feat: Add Thing",
        "feat: add thing.",
        "Added a thing",
        "wip: x",
        "feat: x\nbody",
        "   \n\nbody",
        &format!("feat: {}", "x".repeat(200)),
    ] {
        emitted.extend(check(&l, msg));
    }
    for kind in emitted {
        assert!(
            declared.contains(&kind.as_str()),
            "emitted `{kind}` is not in finding_kinds()"
        );
    }
}
