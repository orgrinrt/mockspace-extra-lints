//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A scratch registry holding the vocabularies the repository ships, for a test that adds and moves
//! tickets the way a person would.
//!
//! The states and the kinds are copied from the pack's `registry/ticket/` and never written again here, so
//! a test reads the values every other test and the board read. A test that needs a kind the
//! repository does not ship adds a row for it to its own scratch and says so.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ticket_model::model::Fields;

use crate::write::{self, AddRequest, MoveRequest};

/// The repository root, from this crate's manifest.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// The vocabulary the pack ships, `registry/` at its root, which a consumer copies into its own
/// registry.
pub fn shipped_registry() -> PathBuf {
    repo_root().join("registry")
}

/// A temporary registry, removed when it goes out of scope.
pub struct Scratch {
    /// The mock directory the registry sits in, as a tool is handed it.
    pub mock: PathBuf,
    /// The registry directory, `<mock>/registry`.
    pub root: PathBuf,
}

impl Scratch {
    /// A scratch whose `ticket/` holds the committed states and kinds and no ticket.
    pub fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let mock = std::env::temp_dir().join(format!(
            "tickets-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&mock);
        let root = mock.join("registry");
        let dir = root.join("ticket");
        std::fs::create_dir_all(&dir).expect("a scratch registry");
        for f in ["states.toml", "kinds.toml"] {
            std::fs::copy(shipped_registry().join("ticket").join(f), dir.join(f))
                .expect("the committed vocabulary");
        }
        Scratch {
            mock,
            root,
        }
    }

    /// Appends a row of text to a file of the scratch, for a fixture the repository does not ship.
    pub fn append(&self, rel: &str, text: &str) {
        use std::io::Write as _;
        let path = self.root.join(rel);
        std::fs::create_dir_all(path.parent().expect("a directory")).expect("a directory");
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("a file");
        writeln!(f, "{text}").expect("written");
    }

    /// The text of a file of the scratch.
    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root.join(rel)).expect("a file")
    }

    /// Adds a ticket as `by` at `at`, with these fields, in its kind's initial state.
    pub fn add(&self, title: &str, fields: &[(&str, &[&str])]) -> Result<String, String> {
        self.add_as(title, "task", None, "", fields)
    }

    /// Adds a ticket with a kind, a state, a holder and fields.
    pub fn add_as(
        &self,
        title: &str,
        kind: &str,
        state: Option<&str>,
        holder: &str,
        fields: &[(&str, &[&str])],
    ) -> Result<String, String> {
        write::add(&self.root, &AddRequest {
            title:  title.to_string(),
            id:     None,
            kind:   kind.to_string(),
            state:  state.map(String::from),
            holder: holder.to_string(),
            by:     "tester".to_string(),
            at:     "2026-10-23T10:00:00Z".to_string(),
            why:    String::new(),
            fields: fields
                .iter()
                .map(|(k, v)| {
                    (
                        (*k).to_string(),
                        v.iter().map(|s| (*s).to_string()).collect(),
                    )
                })
                .collect(),
        })
    }

    /// Records links on a ticket as `tester` at `at`, and moves it to `to` when that is not empty.
    pub fn links(
        &self,
        ticket: &str,
        to: &str,
        holder: Option<&str>,
        links: &[(&str, &[&str])],
    ) -> Result<String, String> {
        write::move_ticket(&self.root, &MoveRequest {
            ticket:    ticket.to_string(),
            to:        to.to_string(),
            links:     links
                .iter()
                .map(|(k, v)| {
                    (
                        (*k).to_string(),
                        v.iter().map(|s| (*s).to_string()).collect(),
                    )
                })
                .collect(),
            holder:    holder.map(String::from),
            release:   false,
            by:        "tester".to_string(),
            at:        "2026-10-23T13:00:00Z".to_string(),
            why:       String::new(),
            answer:    String::new(),
            picks:     Vec::new(),
            tags:      Default::default(),
            member:    None,
            successor: None,
        })
    }

    /// Moves a ticket as `tester` at `at`.
    pub fn mv(
        &self,
        ticket: &str,
        to: &str,
        holder: Option<&str>,
        at: &str,
    ) -> Result<String, String> {
        write::move_ticket(&self.root, &MoveRequest {
            ticket:    ticket.to_string(),
            to:        to.to_string(),
            links:     Fields::new(),
            holder:    holder.map(String::from),
            release:   false,
            by:        "tester".to_string(),
            at:        at.to_string(),
            why:       String::new(),
            answer:    String::new(),
            picks:     Vec::new(),
            tags:      Default::default(),
            member:    None,
            successor: None,
        })
    }

    /// Moves a ticket as `tester` with an answer and the options picked, as a question is closed, at
    /// a time after every other the tests use, so it is the newest move.
    pub fn answer(
        &self,
        ticket: &str,
        to: &str,
        answer: &str,
        picks: &[&str],
    ) -> Result<String, String> {
        write::move_ticket(&self.root, &MoveRequest {
            ticket:    ticket.to_string(),
            to:        to.to_string(),
            links:     Fields::new(),
            holder:    None,
            release:   false,
            by:        "tester".to_string(),
            at:        "2026-10-27T10:00:00Z".to_string(),
            why:       String::new(),
            answer:    answer.to_string(),
            picks:     picks.iter().map(|p| (*p).to_string()).collect(),
            member:    None,
            successor: None,
            tags:      Default::default(),
        })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.mock);
    }
}

/// Where a table of branches stands against the trunk, for a test that asks the tool about git
/// without a repository. A branch the table lacks is `Unknown`.
#[derive(Default)]
pub struct Branches(pub std::collections::BTreeMap<String, crate::landing::Landing>);

impl Branches {
    /// The table with `branch` standing at `landing`.
    pub fn with(mut self, branch: &str, landing: crate::landing::Landing) -> Self {
        self.0.insert(branch.to_string(), landing);
        self
    }
}

impl crate::landing::Trunk for Branches {
    fn landing(&self, branch: &str) -> crate::landing::Landing {
        self.0
            .get(branch)
            .cloned()
            .unwrap_or(crate::landing::Landing::Unknown)
    }
}

impl Scratch {
    /// A stretch and two epochs in it, the spans a member row can name.
    pub fn with_spans(self) -> Self {
        self.append(
            "plan/spans.toml",
            "[[plan_span]]\nid = \"stretch\"\nplanned = false\n\n[[plan_span]]\nid = \"plan\"\nplanned = true\nwithin = \"stretch\"\nowner = \"coord\"\n\n[[plan_span]]\nid = \"fixes\"\nplanned = true\nwithin = \"stretch\"\nowner = \"render-coord\"\n",
        );
        self
    }

    /// Puts a ticket in a span or lets it leave, and sets its standing, as `tester` at `at`.
    pub fn member(
        &self,
        ticket: &str,
        span: Option<&str>,
        core: Option<bool>,
        at: &str,
    ) -> Result<String, String> {
        write::move_ticket(&self.root, &MoveRequest {
            ticket: ticket.to_string(),
            by: "tester".to_string(),
            at: at.to_string(),
            member: Some(write::MemberWrite {
                span: span.map(String::from),
                core,
            }),
            ..MoveRequest::default()
        })
    }
}
