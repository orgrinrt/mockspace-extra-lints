//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What a move gives: the answer and the picks it closes a question with, and what the ticket and its
//! kind ask of them.

use crate::model::{Kind, Move, Ticket};

/// What a move asks of what it gives: a move to a state that satisfies gives one of its kind's
/// `satisfied_by` where the kind lists any, and a move to any other state gives no answer and no
/// picks, since only a ticket's satisfying is answered. A pick is one of `t`'s options word for word.
///
/// # Errors
///
/// The sentence naming the pick that is no option, or the answer or picks a state does not take.
pub fn gives(t: &Ticket, kind: &Kind, satisfies: bool, m: &Move) -> Result<(), String> {
    if let Some(p) = m.picks.iter().find(|p| !t.many("options").contains(p)) {
        return Err(format!(
            "`{p}` is not one of the options `{}` was put with",
            t.id
        ));
    }
    if satisfies {
        if !kind.satisfied_by.is_empty() && !kind.satisfied_by.iter().any(|f| m.gives(f)) {
            return Err(format!(
                "a `{}` ticket goes to `{}` with its {}",
                kind.id,
                m.to,
                kind.satisfied_by
                    .iter()
                    .map(|f| format!("`{f}`"))
                    .collect::<Vec<_>>()
                    .join(" or ")
            ));
        }
    } else if m.gives("answer") || m.gives("picks") {
        return Err(format!(
            "`{}` does not satisfy the ticket, so a move to it carries no answer and no picks",
            m.to
        ));
    }
    Ok(())
}

/// Whether the picks of a move made now fit the ticket's `choice`: several only where it says `many`.
/// A move already made is not judged by this again, since questions closed before the field existed took
/// several picks.
///
/// # Errors
///
/// The sentence naming the ticket and how many picks the move gives.
pub fn fit_the_choice(t: &Ticket, m: &Move) -> Result<(), String> {
    if m.picks.len() > 1 && !t.takes_many() {
        return Err(format!(
            "`{}` takes one pick and the move gives {}: its `choice` is not `{}`",
            t.id,
            m.picks.len(),
            crate::choice::MANY
        ));
    }
    Ok(())
}
