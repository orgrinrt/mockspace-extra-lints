//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Whether a row of the canon binds, read from its `standing` and its stamp.
//!
//! This file declares no lint. The canon lints and the canon tool read a row's standing through it, so
//! the set of words, which of them bind and what a stamp does to a row are named once.

use mockspace::RegistryView;

use crate::canon_rows::{list, text};

/// The two fields a provisional stamp writes on a row: who stamped it (`coordinators` or `experts`) and
/// the stamp itself (what, why, where it is recorded, the question ticket that asks the maintainer). The
/// shape is the one the `a-provisional-stamp-is-not-a-ratification` lint holds rows to.
pub const STAMP_FIELDS: [&str; 2] = ["stamped_by", "stamp"];

/// Why a stamped row binds nothing, as `check` says it.
pub const STAMPED: &str =
    "it carries a stamp, which is never a ratification: it binds nothing until the maintainer ratifies it";

/// Whether the maintainer has ratified a tenet or demoted it, read from its `standing`. The set is the one
/// `mockspace.toml` declares for the field, closed there, and named here once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// It matters now and the maintainer has not ratified it yet: the best current statement, never
    /// cited as settled.
    Provisional,
    /// The maintainer went through it on a named date, with the answer in the row's `answers`.
    Ratified,
    /// The maintainer does not go through it now: kept as reference for a later canon of its domain, binding
    /// nothing.
    Demoted,
    /// The field is absent or holds a word outside the set, which the schema refuses; read so a test
    /// over the committed rows can say which.
    Unstated,
}

impl Standing {
    /// The words of the set, as a row writes them.
    pub const WORDS: [&'static str; 3] = ["provisional", "ratified", "demoted"];

    /// The standing a row's `standing` field spells.
    pub const fn of(word: &str) -> Self {
        match word.as_bytes() {
            b"provisional" => Self::Provisional,
            b"ratified" => Self::Ratified,
            b"demoted" => Self::Demoted,
            _ => Self::Unstated,
        }
    }

    /// The word a listing shows for it.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Provisional => "provisional",
            Self::Ratified => "ratified",
            Self::Demoted => "demoted",
            Self::Unstated => "unstated",
        }
    }

    /// Whether a row of this standing binds, where nothing supersedes it and nothing stamps it. A demoted
    /// row binds nothing; a row whose standing is unstated is read as binding, so a misspelt word never
    /// lets a row slip out of the checks.
    pub const fn binds(self) -> bool {
        !matches!(self, Self::Demoted)
    }

    /// Why a row of this standing binds nothing, as `check` says it; empty for a standing that binds.
    pub const fn unbound(self) -> &'static str {
        match self {
            Self::Demoted => "kept as reference for a later canon of its domain, it binds nothing",
            Self::Provisional | Self::Ratified | Self::Unstated => "",
        }
    }
}

/// Whether the row `q` carries a provisional stamp: either stamp field is written. A stamp keeps the row's
/// standing and is never a ratification, so a stamped row binds nothing whatever its standing says.
pub fn stamped(reg: &RegistryView, q: &str) -> bool {
    STAMP_FIELDS
        .iter()
        .any(|f| text(reg, q, f).is_some_and(|v| !v.is_empty()))
}

/// Whether the row `q` binds: nothing supersedes it, nothing stamps it and its standing binds.
pub fn binding(reg: &RegistryView, q: &str) -> bool {
    list(reg, q, "superseded_by").is_empty()
        && !stamped(reg, q)
        && Standing::of(text(reg, q, "standing").unwrap_or("")).binds()
}
