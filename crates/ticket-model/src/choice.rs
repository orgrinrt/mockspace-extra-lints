//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The `choice` field of a ticket: whether the question it puts takes one pick or several. The value is
//! `one` or `many`, and a ticket that says nothing takes one.

/// The field of a ticket row that says how many options a question is answered with.
pub const FIELD: &str = "choice";

/// The value that lets a question be answered with several options.
pub const MANY: &str = "many";

/// The value that lets a question be answered with one, which is also what no value means.
pub const ONE: &str = "one";

/// Whether a ticket holding these `choice` values and `options` says something that stands: the one value
/// is `one` or `many`, and a ticket that says it holds options to choose among.
///
/// # Errors
///
/// The sentence naming the value, or the lack of options.
pub fn check(choice: &[String], options: &[String]) -> Result<(), String> {
    let Some(value) = choice.first() else {
        return Ok(());
    };
    if value != ONE && value != MANY {
        return Err(format!(
            "`choice` is `{value}`, and a question takes `{ONE}` pick or `{MANY}`"
        ));
    }
    if options.is_empty() {
        return Err(format!(
            "`choice` is `{value}` on a ticket with no `options`, so there is nothing to choose among"
        ));
    }
    Ok(())
}
