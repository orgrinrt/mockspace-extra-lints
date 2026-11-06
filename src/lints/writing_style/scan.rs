/// Every inline-emphasis run in a body, as a line number and the marker used.
///
/// One entry per line rather than per run, because a line carrying three bold
/// spans is one sentence to rewrite and three findings would say so three times.
///
/// What counts: a `**` or `__` pair for bold, and a `*` or `_` pair for italic.
/// Every form requires both ends to sit against a word, so `*this*` and
/// `**this**` are emphasis while `a * b`, `**a + **b` and a bare `---` rule are
/// not. A markdown list's leading `*` never pairs and so never fires.
///
/// One identifier carrying an underscore does not pair either, since a single
/// `snake_case` or `_unused` has one marker. Two on a line do: `_private and
/// public_` reads as an italic run and is reported. That is a known cost of
/// pairing by shape rather than by parsing, and the tests pin both halves so
/// the boundary is where somebody put it rather than where it drifted to.
///
/// Runs over prose that has already had code fences and inline code spans
/// stripped, so a doubled star inside backticks does not reach here.
pub(super) fn emphasis_runs(prose: &str) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    for (index, line) in prose.lines().enumerate() {
        let marker = ["**", "__", "*", "_"]
            .into_iter()
            .find(|m| word_paired(line, m));
        if let Some(marker) = marker {
            out.push((index + 1, marker));
        }
    }
    out
}

/// Whether a line carries an opening and a closing run of `marker`, each
/// against a word.
///
/// The word-adjacency requirement is what separates emphasis from arithmetic,
/// an identifier, a pointer type and a double deref. `**a + **b` opens twice
/// and never closes, so it does not fire; that shape only reaches here at all
/// because plain `//` comments are in scope.
///
/// A run longer than the marker belongs to the longer marker, or to a
/// horizontal rule, so `**bold**` is reported once as bold rather than also as
/// italic, and `***` is neither.
pub(super) fn word_paired(line: &str, marker: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let m: Vec<char> = marker.chars().collect();
    let width = m.len();
    let mut opened = false;
    let mut i = 0;
    while i + width <= chars.len() {
        if chars[i .. i + width] != m[..] {
            i += 1;
            continue;
        }
        let run = run_len(&chars, i, m[0]);
        if run != width {
            i += run;
            continue;
        }
        let before = i.checked_sub(1).and_then(|p| chars.get(p)).copied();
        let after = chars.get(i + width).copied();
        let opens = after.is_some_and(|c| c.is_alphanumeric())
            && before.is_none_or(|c| !c.is_alphanumeric());
        let closes = before.is_some_and(|c| c.is_alphanumeric())
            && after.is_none_or(|c| !c.is_alphanumeric());
        if opened && closes {
            return true;
        }
        if opens {
            opened = true;
        }
        i += width;
    }
    false
}

/// How many `c` in a row start at `start`.
pub(super) fn run_len(chars: &[char], start: usize, c: char) -> usize {
    let mut n = 0;
    while chars.get(start + n) == Some(&c) {
        n += 1;
    }
    n
}

pub(super) fn line_of_first_match(lines: &[&str], needle: &str) -> usize {
    let lower_needle = needle.to_lowercase();
    for (i, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains(&lower_needle) {
            return i + 1;
        }
    }
    1
}

/// Strip fenced code blocks and inline code spans from a markdown body.
/// Used for prose-level checks so code examples and quoted tokens don't
/// trip the heuristics.
pub(super) fn strip_code_spans(content: &str) -> String {
    let fenced = fenced_lines(content);
    let mut out = String::with_capacity(content.len());
    for (index, line) in content.lines().enumerate() {
        if fenced[index] {
            out.push('\n');
            continue;
        }
        let mut in_code = false;
        for ch in line.chars() {
            match ch {
                '`' => in_code = !in_code,
                _ if in_code => out.push(' '),
                c => out.push(c),
            }
        }
        out.push('\n');
    }
    out
}

/// Which lines a fenced block covers, the fence markers included.
///
/// Markers are paired off in order, so a body ending on an unclosed fence has
/// one marker left over and that one covers nothing. Toggling a flag instead
/// blanks every line after it, which is the worst shape a check can take: the
/// caller reads zero findings and nothing says the tail was never looked at.
/// One unterminated fence in one comment is enough to do it, and a rust file
/// carrying a `// ```" in a doc example is how it arrives.
pub(super) fn fenced_lines(content: &str) -> Vec<bool> {
    let markers: Vec<usize> = content
        .lines()
        .enumerate()
        .filter(|(_, l)| l.trim_start().starts_with("```"))
        .map(|(i, _)| i)
        .collect();
    let mut fenced = vec![false; content.lines().count()];
    for pair in markers.chunks_exact(2) {
        for line in &mut fenced[pair[0] ..= pair[1]] {
            *line = true;
        }
    }
    fenced
}

pub(super) fn count_exclamations_in_prose(content: &str) -> usize {
    let fenced = fenced_lines(content);
    let mut count = 0;
    for (index, line) in content.lines().enumerate() {
        if fenced[index] {
            continue;
        }
        // Skip inline code `...` spans roughly.
        let mut in_code = false;
        for ch in line.chars() {
            match ch {
                '`' => in_code = !in_code,
                '!' if !in_code => count += 1,
                _ => {},
            }
        }
    }
    count
}

pub(super) fn opens_with_flat_bullet_list(lines: &[&str]) -> bool {
    // Find the first 3 top-level sections (## or lower).
    let mut section_count = 0;
    let mut scan_from = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("# ") || line.starts_with("## ") {
            section_count += 1;
            if section_count == 1 {
                scan_from = i + 1;
            }
            if section_count > 3 {
                break;
            }
        }
    }
    // Within those top sections, look at the first non-blank non-heading content.
    // If it's a flat bullet list (no sub-bullets, all `- something`), flag it.
    let mut bullets_at_top = 0;
    let mut first_content_seen = false;
    for line in &lines[scan_from .. lines.len().min(scan_from + 60)] {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("#") {
            // New section; stop scanning the previous section's body.
            if first_content_seen {
                break;
            }
            continue;
        }
        if t.starts_with("- ") || t.starts_with("* ") {
            bullets_at_top += 1;
            first_content_seen = true;
        } else if t.starts_with("|") {
            // Tables are allowed if multi-col and multi-row. Don't flag.
            return false;
        } else {
            let _ = first_content_seen;
            break;
        }
    }
    bullets_at_top >= 4
}

pub(super) fn count_label_colon_bullets(lines: &[&str]) -> usize {
    let mut n = 0;
    for line in lines {
        let t = line.trim_start();
        let rest = match t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            Some(r) => r,
            None => continue,
        };
        // Pattern: `<label>: <one short line>` where the colon is in the first
        // half of the content and there are no nested bullets.
        if let Some(colon_pos) = rest.find(':') {
            let before = &rest[.. colon_pos];
            let after = rest[colon_pos + 1 ..].trim();
            let is_short_label = before.len() < 40 && !before.contains(' ');
            let short_after = after.len() < 80 && !after.is_empty();
            if is_short_label && short_after {
                n += 1;
            }
        }
    }
    n
}

