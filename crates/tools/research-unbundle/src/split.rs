//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Cutting a long markdown file along its `## ` headings.

/// The parts a file is written as: the file itself when it fits, else `<stem>.md`, `<stem>-2.md`, and so
/// on, each at most `limit` lines.
///
/// Only markdown is cut. The cuts fall before a `## ` heading, taking as many whole sections into a
/// part as fit; a section longer than `limit` is cut at the last blank line within it, and where it has
/// none at the limit, so no part is ever over.
pub fn parts(name: &str, body: &str, limit: usize) -> Vec<(String, String)> {
    let lines: Vec<&str> = body.lines().collect();
    if !name.ends_with(".md") || lines.len() <= limit {
        return vec![(name.to_string(), body.to_string())];
    }
    let mut chunks: Vec<Vec<&str>> = Vec::new();
    let mut fence = false;
    for line in &lines {
        if line.starts_with("```") {
            fence = !fence;
        }
        if !fence && line.starts_with("## ") || chunks.is_empty() {
            chunks.push(Vec::new());
        }
        if let Some(c) = chunks.last_mut() {
            c.push(line);
        }
    }
    // sections longer than the limit are cut first, so the packing below only meets whole pieces
    let mut pieces: Vec<Vec<&str>> = Vec::new();
    for chunk in chunks {
        let mut rest: &[&str] = &chunk;
        while rest.len() > limit {
            let window = &rest[.. limit];
            let at = window
                .iter()
                .rposition(|l| l.trim().is_empty())
                .filter(|&i| i > 0)
                .map_or(limit, |i| i + 1);
            pieces.push(rest[.. at].to_vec());
            rest = &rest[at ..];
        }
        pieces.push(rest.to_vec());
    }
    let mut out: Vec<Vec<&str>> = Vec::new();
    for piece in pieces {
        match out.last_mut() {
            Some(last) if last.len() + piece.len() <= limit => last.extend(piece),
            _ => out.push(piece),
        }
    }
    let stem = name.strip_suffix(".md").unwrap_or(name);
    out.into_iter()
        .enumerate()
        .map(|(i, part)| {
            let n = if i == 0 { name.to_string() } else { format!("{stem}-{}.md", i + 1) };
            (n, format!("{}\n", part.join("\n")))
        })
        .collect()
}
