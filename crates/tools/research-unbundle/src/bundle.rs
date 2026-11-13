//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Reading a bundle: `=== FILE: <name> ===` and the lines under it, until the next such line.

/// The files of a bundle in order, each name with its text, which ends in one newline.
///
/// # Errors
///
/// When text comes before the first header, no file is named, a name is unsafe or given twice, or a
/// file is empty.
pub fn parse(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut files: Vec<(String, Vec<&str>)> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if let Some(name) = header(line) {
            if !safe(name) {
                return Err(format!(
                    "line {}: `{name}` is not a name a note's file may have",
                    i + 1
                ));
            }
            if files.iter().any(|(n, _)| n == name) {
                return Err(format!("line {}: `{name}` is given twice", i + 1));
            }
            files.push((name.to_string(), Vec::new()));
        } else if let Some((_, body)) = files.last_mut() {
            body.push(line);
        } else if !line.trim().is_empty() {
            return Err(format!(
                "line {}: text before the first `=== FILE: <name> ===` line belongs to no file",
                i + 1
            ));
        }
    }
    if files.is_empty() {
        return Err("the bundle names no file: no `=== FILE: <name> ===` line".to_string());
    }
    files
        .into_iter()
        .map(|(name, mut body)| {
            while body.last().is_some_and(|l| l.trim().is_empty()) {
                body.pop();
            }
            if body.is_empty() {
                return Err(format!("`{name}` is empty"));
            }
            Ok((name, format!("{}\n", body.join("\n"))))
        })
        .collect()
}

/// The name in a header line, where the line is one.
fn header(line: &str) -> Option<&str> {
    line.strip_prefix("=== FILE: ")?.strip_suffix(" ===")
}

/// A relative path of plain segments: letters, digits, `.`, `_` and `-`, none empty, none `.` or `..`.
fn safe(name: &str) -> bool {
    !name.is_empty()
        && name.split('/').all(|seg| {
            !seg.is_empty()
                && seg != "."
                && seg != ".."
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        })
}
