//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Whether the tenets and subjects a ticket links to are rows of the canon.
//!
//! The ticket model crate reads no canon, so the ids the canon holds come in from the tool that reads it: the
//! ticket makers refuse a link to a row that is not there, and `tickets check` names the ones a hand edit
//! left (tickets_check_accepts_tenets_and_subjects_that_do_not_exist).

use std::collections::BTreeSet;

use ticket_model::model::{Fields, Store};
use ticket_model::rules::Finding;
use mockspace::RegistryView;

/// The links of a ticket that name a canon row: `related` entries as `tenet::<id>` or `subject::<id>`, and
/// the `tenets` field, whose entries are `tenet::<id>` or a bare id.
const TENET_FIELDS: [&str; 1] = ["tenets"];

/// The tenet and subject ids the canon holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Known {
    pub tenets:   BTreeSet<String>,
    pub subjects: BTreeSet<String>,
}

impl Known {
    /// What the registry view holds.
    pub fn of(registry: &RegistryView) -> Self {
        Known {
            tenets:   canon::tenets(registry)
                .iter()
                .map(|t| t.id.to_string())
                .collect(),
            subjects: canon::subjects(registry)
                .iter()
                .map(|(id, _)| (*id).to_string())
                .collect(),
        }
    }

    /// The entries among these links that name a tenet or a subject the canon does not hold, as written.
    pub fn unknown(&self, fields: &Fields) -> Vec<String> {
        let mut out = Vec::new();
        for entry in fields.get("related").into_iter().flatten() {
            let bad = match entry.split_once("::") {
                Some(("tenet", id)) => !self.tenets.contains(id),
                Some(("subject", id)) => !self.subjects.contains(id),
                _ => false,
            };
            if bad {
                out.push(entry.clone());
            }
        }
        for field in TENET_FIELDS {
            for entry in fields.get(field).into_iter().flatten() {
                let id = entry.strip_prefix("tenet::").unwrap_or(entry);
                if !self.tenets.contains(id) {
                    out.push(entry.clone());
                }
            }
        }
        out
    }

    /// The sentence a maker refuses a request with, or nothing where every link names a row.
    ///
    /// # Errors
    ///
    /// A sentence naming each link the canon does not hold.
    pub fn refuse(&self, fields: &Fields) -> Result<(), String> {
        let bad = self.unknown(fields);
        if bad.is_empty() {
            return Ok(());
        }
        Err(format!(
            "{} no row of the canon: `cargo mock canon subjects` and `cargo mock canon find <term>` name the \
             ones there are",
            bad.iter()
                .map(|b| format!("`{b}` is"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    /// Every ticket whose links name a tenet or subject the canon does not hold.
    pub fn findings(&self, store: &Store) -> Vec<Finding> {
        store
            .tickets
            .values()
            .filter_map(|t| {
                let bad = self.unknown(&t.fields);
                (!bad.is_empty()).then(|| {
                    Finding {
                        row:     format!("ticket::{}", t.id),
                        message: format!(
                            "it links {}, which no row of the canon carries",
                            bad.iter()
                                .map(|b| format!("`{b}`"))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    }
                })
            })
            .collect()
    }
}
