//! `tickets answerable`: for every open question, what the repository index already holds that
//! could answer it. It answers nothing itself: a person reads the candidates before a question stands.

use std::fmt::Write as _;

use ticket_model::Store;

use crate::view::{Filter, Row};

/// How many candidates stand beside a question.
pub const PER_QUESTION: usize = 3;

pub use corpus::repo::Found as Candidate;

/// The filter that selects the open questions, narrowed by what the caller already asked for.
#[must_use]
pub fn open_questions(base: &Filter) -> Filter {
    Filter {
        kind: if base.kind.is_empty() {
            vec!["question".to_string()]
        } else {
            base.kind.clone()
        },
        state: if base.state.is_empty() {
            vec!["waiting".to_string()]
        } else {
            base.state.clone()
        },
        ..base.clone()
    }
}

/// The words a question is searched by: its title and each option.
fn words(store: &Store, id: &str) -> String {
    let Some(t) = store.tickets.get(id) else {
        return String::new();
    };
    let mut out = t.one("title").to_string();
    for o in t.many("options") {
        out.push(' ');
        out.push_str(o);
    }
    out
}

/// One block per question, its candidates under it, and how many questions were asked about.
#[must_use]
pub fn report(
    store: &Store,
    rows: &[Row],
    ask: &dyn Fn(&str) -> Vec<Candidate>,
) -> (String, usize) {
    let mut out = String::new();
    for r in rows {
        let _ = writeln!(out, "{}  {}", r.id, r.title);
        let found = ask(&words(store, &r.id));
        if found.is_empty() {
            out.push_str("    no candidate in the index\n");
        }
        for c in found.iter().take(PER_QUESTION) {
            let _ = writeln!(
                out,
                "    {}:{}  ({})  {}",
                c.path, c.line, c.class, c.excerpt
            );
        }
    }
    (out, rows.len())
}
