use super::*;

/// The word is written `§` here, so this file holds no marker of its own.
fn t(s: &str) -> String {
    s.replace('§', WORD)
}

fn one(text: &str, kind: Kind) -> Marker {
    let all = scan(&t(text), kind);
    assert_eq!(all.len(), 1, "{all:?}");
    all.into_iter().next().unwrap()
}

#[test]
fn a_tagged_marker_gives_its_tag_and_its_reason() {
    let m = one("fn a() {} // §(stand-in): a first take\n", Kind::Slashes);
    assert_eq!(m.tag.as_deref(), Some("stand-in"));
    assert!(m.colon);
    assert_eq!(m.reason, "a first take");
    assert_eq!(m.line, 1);
}

#[test]
fn a_mention_without_a_colon_in_prose_is_not_a_marker() {
    let text = "Marked with a § in the source, and §(canon silent) elsewhere.\n";
    assert!(scan(&t(text), Kind::Prose).is_empty());
    assert!(scan(&t("// the § in `Species::draw`\n"), Kind::Slashes).is_empty());
}

#[test]
fn the_word_inside_quotation_marks_or_another_word_is_not_a_marker() {
    assert!(scan(&t("let s = \"§(x): y\";\n"), Kind::Slashes).is_empty());
    assert!(scan(&t("// §s: left over\n"), Kind::Slashes).is_empty());
    assert!(scan(&t("// NOT§: left over\n"), Kind::Slashes).is_empty());
}

#[test]
fn a_comment_opening_with_the_word_and_no_colon_is_a_marker_that_lost_it() {
    let m = one("// § fix this later\n", Kind::Slashes);
    assert!(!m.colon);
    assert_eq!(m.tag, None);
    let m = one("# §(op) later\n", Kind::Hash);
    assert!(!m.colon);
    assert_eq!(m.tag.as_deref(), Some("op"));
}

#[test]
fn in_toml_only_a_comment_is_read() {
    assert!(scan(&t("text = \"see §(x): y\"\n"), Kind::Toml).is_empty());
    assert_eq!(scan(&t("# §(x): y\nkey = 1\n"), Kind::Toml).len(), 1);
    assert_eq!(scan(&t("key = 1 # §: y\n"), Kind::Toml).len(), 1);
    assert!(scan(&t("key = \"a # §: y\"\n"), Kind::Toml).is_empty());
}

#[test]
fn a_backticked_marker_in_a_design_is_read_with_its_tag() {
    let m = one("`§(canon silent)`: a band about the light\n", Kind::Prose);
    assert_eq!(m.tag.as_deref(), Some("canon silent"));
    assert_eq!(m.reason, "a band about the light");
}

#[test]
fn the_reason_carries_on_through_the_comment_and_stops_at_its_end() {
    let text =
        "// §(op): the first line\n// and the second\n//\n// not part of it\nlet x = 1;\n";
    assert_eq!(
        one(text, Kind::Slashes).reason,
        "the first line and the second"
    );
    let text = "/// §: a doc\n/// comment\nfn f() {}\n";
    assert_eq!(one(text, Kind::Slashes).reason, "a doc comment");
}

#[test]
fn the_reason_of_a_prose_marker_is_its_paragraph() {
    let text = "§(canon silent): how a roof pitches,\nand where it ends.\n\nNext paragraph.\n";
    assert_eq!(
        one(text, Kind::Prose).reason,
        "how a roof pitches, and where it ends."
    );
    let text = "<!-- §: a note\nacross two lines -->\ntext\n";
    assert_eq!(one(text, Kind::Prose).reason, "a note across two lines");
}

#[test]
fn two_markers_on_one_line_each_keep_their_own_words() {
    let all = scan(&t("a; §(x): first part; §(y): second part\n"), Kind::Prose);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].reason, "first part;");
    assert_eq!(all[1].reason, "second part");
}

#[test]
fn rewriting_replaces_the_head_and_leaves_the_rest_byte_for_byte() {
    let text = t("a // §(stand-in): keep  this  spacing\n// § no colon\n");
    let all = scan(&text, Kind::Slashes);
    assert_eq!(all.len(), 2);
    let pairs: Vec<(&Marker, &str)> = all.iter().map(|m| (m, "the_ticket")).collect();
    let out = rewrite(&text, &pairs);
    assert_eq!(
        out,
        t("a // §(the_ticket): keep  this  spacing\n// §(the_ticket): no colon\n")
    );
    // and what it wrote is read back as markers citing the ticket
    assert!(
        scan(&out, Kind::Slashes)
            .iter()
            .all(|m| m.tag.as_deref() == Some("the_ticket"))
    );
    // a backticked head in a design keeps its backticks
    let text = t("`§(canon silent)`: x\n");
    let all = scan(&text, Kind::Prose);
    let out = rewrite(&text, &[(&all[0], "the_ticket")]);
    assert_eq!(out, t("`§(the_ticket)`: x\n"));
}

#[test]
fn reciting_hands_the_markers_of_one_ticket_to_another_and_leaves_the_rest() {
    let text = t(
        "// §(old): first\nlet a = 1; // §(old_two): another ticket\n// §(old): second\n// §(other): a third\n",
    );
    let out = recite(&text, Kind::Slashes, "old", "new").unwrap();
    assert_eq!(
        out,
        t(
            "// §(new): first\nlet a = 1; // §(old_two): another ticket\n// §(new): second\n// §(other): a third\n"
        )
    );
}

#[test]
fn reciting_a_text_with_no_marker_citing_the_ticket_gives_nothing() {
    // the control for the test above: the id in prose, in a string, in a longer id and in no marker at all
    assert_eq!(
        recite("the ticket old is closed\n", Kind::Prose, "old", "new"),
        None
    );
    assert_eq!(
        recite(&t("let s = \"§(old): x\";\n"), Kind::Slashes, "old", "new"),
        None
    );
    assert_eq!(
        recite(
            &t("// §(older): x\n// §(gold): y\n"),
            Kind::Slashes,
            "old",
            "new"
        ),
        None
    );
    assert_eq!(
        recite(&t("// § no ticket\n"), Kind::Slashes, "old", "new"),
        None
    );
    assert_eq!(recite("fn a() {}\n", Kind::Slashes, "old", "new"), None);
}

#[test]
fn reciting_reads_the_tag_as_the_lint_does_trimmed_and_whole() {
    let out = recite(
        &t("// §( old ): spaced\n// §(older): longer\n"),
        Kind::Slashes,
        "old",
        "new",
    )
    .unwrap();
    assert_eq!(out, t("// §(new): spaced\n// §(older): longer\n"));
}

#[test]
fn reciting_works_in_every_kind_of_file_and_adds_the_colon_a_marker_lost() {
    let cases = [
        (
            Kind::Toml,
            "# §(old): in a config\nkey = 1\n",
            "# §(new): in a config\nkey = 1\n",
        ),
        (
            Kind::Hash,
            "# §(old): in a script\n",
            "# §(new): in a script\n",
        ),
        (
            Kind::Prose,
            "`§(old)`: in a design\n",
            "`§(new)`: in a design\n",
        ),
        (
            Kind::Prose,
            "<!-- §(old): in a note -->\n",
            "<!-- §(new): in a note -->\n",
        ),
        (
            Kind::Slashes,
            "// §(old) lost its colon\n",
            "// §(new): lost its colon\n",
        ),
    ];
    for (kind, from, to) in cases {
        assert_eq!(recite(&t(from), kind, "old", "new"), Some(t(to)), "{from}");
    }
}

#[test]
fn what_reciting_wrote_is_read_back_as_markers_of_the_successor_alone() {
    let text = t("// §(old): a\n// §(old): b\n");
    let out = recite(&text, Kind::Slashes, "old", "new").unwrap();
    let all = scan(&out, Kind::Slashes);
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|m| m.tag.as_deref() == Some("new")));
    assert_eq!(recite(&out, Kind::Slashes, "old", "new"), None);
}

#[test]
fn files_are_in_scope_by_where_they_stand() {
    assert_eq!(scope_of("mock/crates/a/src/lib.rs", &layout()), Some(Scope::Live));
    assert_eq!(scope_of("content/kaski/x.toml", &layout()), Some(Scope::Live));
    assert_eq!(scope_of("mock/research/n/NOTES.md", &layout()), Some(Scope::Record));
    assert_eq!(
        scope_of("mock/design_rounds/2026/x.md", &layout()),
        Some(Scope::Record)
    );
    assert_eq!(scope_of("docs/X.md", &layout()), None);
    assert_eq!(scope_of("mock/vendor/a/src/lib.rs", &layout()), None);
    assert_eq!(scope_of("mock/crates/a/node_modules/x.js", &layout()), None);
}

#[test]
fn kinds_follow_the_extension_and_a_shebang() {
    assert_eq!(kind_of("a/b.rs", ""), Some(Kind::Slashes));
    assert_eq!(kind_of("a/b.toml", ""), Some(Kind::Toml));
    assert_eq!(kind_of("a/DESIGN.md.tmpl", ""), Some(Kind::Prose));
    assert_eq!(
        kind_of("a/page/build", "#!/usr/bin/env nutshell"),
        Some(Kind::Hash)
    );
    assert_eq!(kind_of("a/b.png", ""), None);
    assert_eq!(kind_of("a/build", "text"), None);
}

/// The layout the fixtures are written in: the mock tree under `mock`, the content under `content`.
fn layout() -> crate::layout::Layout {
    crate::layout::Layout::new("mock", crate::layout::ContentDirs::new(&["content"]))
}
