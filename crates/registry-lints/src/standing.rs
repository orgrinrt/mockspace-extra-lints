//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Whether a row of the canon binds, read from its `standing`.
//!
//! This file declares no lint. The canon lints and the canon tool read a row's standing through it, so
//! the set of words and which of them bind are named once.

use mockspace::RegistryView;

use crate::canon_rows::{list, text};

/// Whether the maintainer has ratified a tenet, holds it pending or demoted it, read from its `standing`. The set is the one
/// `mockspace.toml` declares for the field, closed there, and named here once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// It matters now and the maintainer has not ratified it yet: the best current statement, never
    /// cited as settled.
    Provisional,
    /// The maintainer went through it on a named date, with the answer in the row's `answers`.
    Ratified,
    /// Written down and waiting on the maintainer, who has neither ratified it nor let it stand as the
    /// current statement: shown and searched, binding nothing until it is ratified or made provisional.
    Pending,
    /// The maintainer does not go through it now: kept as reference for a later canon of its domain, binding
    /// nothing.
    Demoted,
    /// The field is absent or holds a word outside the set, which the schema refuses; read so a test
    /// over the committed rows can say which.
    Unstated,
}

impl Standing {
    /// The words of the set, as a row writes them.
    pub const WORDS: [&'static str; 4] = ["provisional", "ratified", "pending", "demoted"];

    /// The standing a row's `standing` field spells.
    pub const fn of(word: &str) -> Self {
        match word.as_bytes() {
            b"provisional" => Self::Provisional,
            b"ratified" => Self::Ratified,
            b"pending" => Self::Pending,
            b"demoted" => Self::Demoted,
            _ => Self::Unstated,
        }
    }

    /// The word a listing shows for it.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Provisional => "provisional",
            Self::Ratified => "ratified",
            Self::Pending => "pending",
            Self::Demoted => "demoted",
            Self::Unstated => "unstated",
        }
    }

    /// Whether a row of this standing binds, where nothing supersedes it. A pending row and a demoted one
    /// bind nothing; a row whose standing is unstated is read as binding, so a misspelt word never lets a
    /// row slip out of the checks.
    pub const fn binds(self) -> bool {
        !matches!(self, Self::Pending | Self::Demoted)
    }

    /// Why a row of this standing binds nothing, as `check` says it; empty for a standing that binds.
    pub const fn unbound(self) -> &'static str {
        match self {
            Self::Pending => {
                "waiting on the maintainer, it binds nothing until it is ratified or made provisional"
            },
            Self::Demoted => "kept as reference for a later canon of its domain, it binds nothing",
            Self::Provisional | Self::Ratified | Self::Unstated => "",
        }
    }
}

/// Whether the row `q` binds: nothing supersedes it and its standing binds.
pub fn binding(reg: &RegistryView, q: &str) -> bool {
    list(reg, q, "superseded_by").is_empty()
        && Standing::of(text(reg, q, "standing").unwrap_or("")).binds()
}
