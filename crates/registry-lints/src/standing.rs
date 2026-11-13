//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Whether a canon row binds, read from its `standing`, and whether it is stamped.
//!
//! This file declares no lint. The canon lints and the canon tool read a row's standing through it, so
//! the set of words, which of them bind and what a stamp is are named once.

use mockspace::RegistryView;

use crate::canon_rows::{list, text};

/// What a stamp is, as `check` says it beside a stamped row.
pub const STAMPED: &str =
    "a stamp is as good as the maintainer's word for now and never a ratification: the row binds as its standing does";

/// Whether the maintainer has ratified a tenet, holds it for himself or demoted it, read from its
/// `standing`. The set is named here once, and a project's schema declares the words of it it uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standing {
    /// It matters now and the maintainer has not ratified it yet: the best current statement, never
    /// cited as settled. A provisional row on a stamp reads the same.
    Provisional,
    /// The maintainer went through it on a named date, with the answer in the row's `answers`.
    Ratified,
    /// Brought to the maintainer, because the written history does not back it or a word of his stands
    /// against it: shown and searched, binding nothing until he backs it.
    Held,
    /// The maintainer does not go through it now: kept as reference for a later canon of its domain, binding
    /// nothing.
    Demoted,
    /// The field is absent or holds a word outside the set, which the schema refuses; read as binding so
    /// a misspelt word never lets a row slip out of the checks, and so a test over the committed rows can
    /// say which.
    Unstated,
}

impl Standing {
    /// The words of the set, as a row writes them.
    pub const WORDS: [&'static str; 4] = ["provisional", "ratified", "held", "demoted"];

    /// The standing a row's `standing` field spells.
    pub const fn of(word: &str) -> Self {
        match word.as_bytes() {
            b"provisional" => Self::Provisional,
            b"ratified" => Self::Ratified,
            b"held" => Self::Held,
            b"demoted" => Self::Demoted,
            _ => Self::Unstated,
        }
    }

    /// The word a listing shows for it.
    pub const fn word(self) -> &'static str {
        match self {
            Self::Provisional => "provisional",
            Self::Ratified => "ratified",
            Self::Held => "held",
            Self::Demoted => "demoted",
            Self::Unstated => "unstated",
        }
    }

    /// Whether a row of this standing binds, where nothing supersedes it. A held row and a demoted one
    /// bind nothing; a row whose standing is unstated is read as binding, so a misspelt word never lets a
    /// row slip out of the checks.
    pub const fn binds(self) -> bool {
        !matches!(self, Self::Held | Self::Demoted)
    }

    /// Why a row of this standing binds nothing, as `check` says it; empty for a standing that binds.
    pub const fn unbound(self) -> &'static str {
        match self {
            Self::Held => "brought to the maintainer, it binds nothing until he backs it",
            Self::Demoted => "kept as reference for a later canon of its domain, it binds nothing",
            Self::Provisional | Self::Ratified | Self::Unstated => "",
        }
    }
}

/// Whether the row `q` carries a provisional stamp: either stamp field is written. A stamp keeps the
/// row's standing and changes nothing about whether it binds; it is never a ratification.
pub fn stamped(reg: &RegistryView, q: &str) -> bool {
    stamp::FIELDS
        .iter()
        .any(|f| text(reg, q, f).is_some_and(|v| !v.is_empty()))
}

/// Whether the row `q` binds: nothing supersedes it and its standing binds.
pub fn binding(reg: &RegistryView, q: &str) -> bool {
    list(reg, q, "superseded_by").is_empty()
        && Standing::of(text(reg, q, "standing").unwrap_or("")).binds()
}
