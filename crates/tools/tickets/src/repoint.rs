//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `ticket-move --repoint-superseded`: every ticket naming a superseded tenet that has one heir is
//! written to name the heir.
//!
//! What binds in a superseded tenet's place is read once, by `canon`, and which links a ticket should hold
//! is `kaski-tickets`'s `repoint`; this is only the write, one links-alone record per ticket changed, each
//! judged by the same rules as any other.

use std::path::Path;

use ticket_model::{Fields, heirs};
use mockspace::RegistryView;

use crate::write::{MoveRequest, move_ticket};

/// The ids of the tickets written to name an heir, in id order. Every ticket is tried; a refusal is
/// returned with the ones written, so a sweep is never half-silent.
pub fn repoint_superseded(
    registry: &Path,
    tenets: &RegistryView,
    by: &str,
    at: &str,
) -> Result<Vec<String>, (Vec<String>, String)> {
    let store = crate::load(registry).map_err(|e| (Vec::new(), e))?;
    let heirs_of = canon::heirs(&canon::tenets(tenets));
    let mut written = Vec::new();
    let mut refused = Vec::new();
    for t in store.tickets.values() {
        let r = heirs::repoint(t, &heirs_of);
        if !r.changed {
            continue;
        }
        let mut links = Fields::new();
        for (field, values) in [("related", r.related), ("tenets", r.tenets)] {
            if !t.many(field).is_empty() {
                links.insert(field.to_string(), values);
            }
        }
        let request = MoveRequest {
            ticket: t.id.clone(),
            links,
            by: by.to_string(),
            at: at.to_string(),
            ..MoveRequest::default()
        };
        match move_ticket(registry, &request) {
            Ok(_) => written.push(t.id.clone()),
            Err(e) => refused.push(format!("`{}`: {e}", t.id)),
        }
    }
    if refused.is_empty() {
        Ok(written)
    } else {
        Err((written, refused.join("\n")))
    }
}
