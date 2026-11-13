//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Finding the tenets a text names.
//!
//! A tenet is cited by its bare id, in prose, in a comment, in backticks or behind `tenet::`, so a
//! name is a whole id bounded by anything that cannot sit inside one. A tenet whose id is the start of
//! another's is not named by the longer one.

use std::collections::BTreeSet;

/// Whether a character can sit inside an id.
fn id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The ids in `known` that `text` names, in the order it first names them, each once.
pub fn named<'a>(text: &str, known: &BTreeSet<&'a str>) -> Vec<&'a str> {
    let mut out: Vec<&'a str> = Vec::new();
    for word in text.split(|c: char| !id_char(c)) {
        if let Some(id) = known.get(word)
            && !out.contains(id)
        {
            out.push(id);
        }
    }
    out
}
