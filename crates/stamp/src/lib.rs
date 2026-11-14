//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The shape of a provisional stamp on a registry row, named once.
//!
//! A stamp lets work proceed on a row while the maintainer is away. It is written in two fields, says
//! which route made it, and names the question ticket that asks the maintainer to make it formal. The
//! lint `a-provisional-stamp-is-not-a-ratification` holds rows to this shape and the canon tool reads
//! it, both through this crate, so a second spelling of either field cannot arise.

/// The field naming the route that stamped the row.
pub const STAMPED_BY: &str = "stamped_by";

/// The field holding the stamp: what was stamped, why, where it is recorded and its question ticket.
pub const STAMP: &str = "stamp";

/// Both fields, in the order a row writes them.
pub const FIELDS: [&str; 2] = [STAMPED_BY, STAMP];

/// The routes onto a stamp, the values `stamped_by` takes: the coordinators converging, two domain
/// experts agreeing after a panel stretch, or the maintainer directing the row onto a stamp himself.
/// The pack's `registry/namespaces.toml` declares the same values, and a test holds the two equal.
pub const ROUTES: [&str; 3] = [COORDINATORS, EXPERTS, MAINTAINER];

/// The route where the coordinators converged on the row.
pub const COORDINATORS: &str = "coordinators";

/// The route where two domain experts agreed after a panel stretch.
pub const EXPERTS: &str = "experts";

/// The route where the maintainer put the row on a stamp himself, his passage cited in the stamp.
pub const MAINTAINER: &str = "maintainer";

/// The word a stamp names its question ticket after, as `ticket` and then the id in backticks.
pub const TICKET: &str = "ticket";

/// The id of the question ticket `stamp` names, the first where it names several, or nothing where it
/// names none. An id is one word in backticks after the word `ticket`, padding trimmed, so an empty
/// id or one with a space inside it is not.
pub fn ticket(stamp: &str) -> Option<&str> {
    stamp.match_indices(TICKET).find_map(|(at, _)| {
        let rest = stamp[at + TICKET.len() ..].trim_start();
        let (id, _) = rest.strip_prefix('`')?.split_once('`')?;
        let id = id.trim();
        (!id.is_empty() && !id.contains(char::is_whitespace)).then_some(id)
    })
}

#[cfg(test)]
mod tests;
