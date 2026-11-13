//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The member row `ticket-move --span` and `--core` append: one fact, which span a ticket is in and
//! whether it is core there, naming the row it replaces.

use kaski_planning::Plan;

use crate::ids::move_id;
use crate::write::{MemberWrite, MoveRequest, quote};

/// The text of the member row the request writes, to append to the ticket's file.
///
/// The row carries what the request states and what the ticket's current member row already says, so
/// a call that only changes `--core` keeps the span, and it names that current row in `over`.
///
/// # Errors
///
/// When the ticket's member rows fork, so there is no one row to write over, or the span named is no
/// span.
pub fn render(plan: &Plan, r: &MoveRequest, w: &MemberWrite) -> Result<String, String> {
    let heads = plan.heads("member", "ticket", &r.ticket);
    if heads.len() > 1 {
        let ids: Vec<&str> = heads.iter().map(|h| h.id.as_str()).collect();
        return Err(format!(
            "`{}` has {} current member rows ({}), two writers over one predecessor; settle them first",
            r.ticket,
            heads.len(),
            ids.join(", ")
        ));
    }
    let head = heads.first();
    let span = match &w.span {
        Some(s) => s.clone(),
        None => head.map(|h| h.one("span").to_string()).unwrap_or_default(),
    };
    if !span.is_empty() && plan.row("plan_span", &span).is_none() {
        return Err(format!("`--span` names `{span}`, which is no span"));
    }
    let core = w.core.map(|c| c.to_string()).or_else(|| {
        head.map(|h| h.one("core").to_string())
            .filter(|c| !c.is_empty())
    });
    let mut attempt = 0;
    let id = loop {
        let id = format!("p{}", move_id(&r.at, &r.by, attempt));
        if plan.row("member", &id).is_none() {
            break id;
        }
        attempt += 1;
    };
    let mut out = format!(
        "[[member]]\nid = {}\nticket = {}\n",
        quote(&id),
        quote(&r.ticket)
    );
    if !span.is_empty() {
        out.push_str(&format!("span = {}\n", quote(&span)));
    }
    if let Some(core) = core {
        out.push_str(&format!("core = {core}\n"));
    }
    if let Some(h) = head {
        out.push_str(&format!("over = {}\n", quote(&h.id)));
    }
    out.push_str(&format!("at = {}\nby = {}\n", quote(&r.at), quote(&r.by)));
    Ok(out)
}
