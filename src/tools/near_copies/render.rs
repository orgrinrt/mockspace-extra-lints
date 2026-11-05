//! The report as text: silent on nothing, detail per finding.

use mockspace_lint_rules::DeclaredType;

use super::compare::{Found, Pair};

/// The report, empty when there is nothing to say.
pub fn report(found: &Found<'_>, limit: usize) -> String {
    let mut out = String::new();
    section(
        &mut out,
        "near-copies",
        "different names, close declarations",
        &found.near_copies,
        limit,
    );
    section(
        &mut out,
        "same name, different shape",
        "what single-source leaves alone",
        &found.same_named,
        limit,
    );
    out
}

fn section(out: &mut String, title: &str, what: &str, pairs: &[Pair<'_>], limit: usize) {
    if pairs.is_empty() {
        return;
    }
    out.push_str(&format!("{title} ({}): {what}\n", pairs.len()));
    for pair in pairs.iter().take(limit) {
        finding(out, pair);
    }
    if pairs.len() > limit {
        out.push_str(&format!(
            "  ... {} more, `--limit` shows them\n",
            pairs.len() - limit
        ));
    }
    out.push('\n');
}

fn finding(out: &mut String, pair: &Pair<'_>) {
    out.push_str(&format!(
        "  {:>3}%  {}  ~  {}\n",
        pair.close.percent(),
        place(pair.first),
        place(pair.second)
    ));
    let close = &pair.close;
    if !close.shared.is_empty() {
        out.push_str(&format!("        shared: {}\n", close.shared.join(", ")));
    }
    for (a, b, body) in &close.renamed {
        let word = if a.chars().all(|c| c.is_ascii_digit()) { "moved" } else { "renamed" };
        out.push_str(&format!("        {word}: {a} -> {b} of {body}\n"));
    }
    if !close.only_first.is_empty() {
        out.push_str(&format!(
            "        only in {}: {}\n",
            pair.first.crate_name,
            close.only_first.join(", ")
        ));
    }
    if !close.only_second.is_empty() {
        out.push_str(&format!(
            "        only in {}: {}\n",
            pair.second.crate_name,
            close.only_second.join(", ")
        ));
    }
    if close.kinds_differ {
        out.push_str(&format!(
            "        keywords differ: {} against {}\n",
            pair.first.kind, pair.second.kind
        ));
    }
    if let Some((first, second)) = &close.frame {
        out.push_str(&format!(
            "        declared differently beside the members: `{first}` against `{second}`\n"
        ));
    }
    if close.generics_differ {
        out.push_str(&format!(
            "        generics differ: `{}` against `{}`\n",
            pair.first.parts.generics, pair.second.parts.generics
        ));
    }
}

/// `struct Name (crate path:line)`.
fn place(t: &DeclaredType) -> String {
    let path = t.path.as_deref().unwrap_or("src/lib.rs");
    format!(
        "{} {} ({} {}:{})",
        t.kind, t.name, t.crate_name, path, t.line
    )
}
