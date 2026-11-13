//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The `options` field of a ticket: one string for each option, a lettered label and its words. An
//! option that holds a further lettered option is several written as one, which the console shows as
//! a single option, so it is refused here.

/// The field of a ticket row that holds the options a question was put with.
pub const FIELD: &str = "options";

/// Whether `option` holds a lettered option after its own start: a comma, optional spaces, one lowercase
/// letter and a colon with a space, as `a: x,b: y` does.
fn holds_another(option: &str) -> bool {
    let b = option.as_bytes();
    (1 .. b.len()).any(|i| {
        if b[i] != b',' {
            return false;
        }
        let rest = &b[i + 1 ..];
        let rest = &rest[rest.iter().take_while(|c| **c == b' ').count() ..];
        matches!(rest, [l, b':', b' ', ..] if l.is_ascii_lowercase())
    })
}

/// The sentence for the first of `options` that holds several, naming it.
///
/// # Errors
///
/// The sentence, when an option holds a further lettered option.
pub fn check(options: &[String]) -> Result<(), String> {
    match options.iter().find(|o| holds_another(o)) {
        Some(bad) => {
            Err(format!(
                "`options` holds `{bad}`, which is several options in one: separate options with a vertical bar"
            ))
        },
        None => Ok(()),
    }
}
