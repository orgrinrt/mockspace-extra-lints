//! A Rust source with its comments and the insides of its literals blanked.
//!
//! The reading of attributes and function heads in `lib.rs` runs over this, so a `#[test]` written
//! inside a string, as a lint's planted fixture is, or inside a comment, is not a test. Every blanked
//! byte becomes a space and a newline stays a newline, so an offset and a line number in the masked
//! text are the same offset and line in the source, and what a literal says is read from the source
//! at the offsets the mask found.

/// The source with each comment and each string or character literal's inside turned to spaces. The
/// quotes of a literal stay, so where it starts and ends can be found.
pub fn masked(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                let end = line_end(bytes, i);
                blank(&mut out, i, end);
                i = end;
            },
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let end = block_comment_end(bytes, i);
                blank(&mut out, i, end);
                i = end;
            },
            b'"' => {
                let end = string_end(bytes, i + 1);
                blank(&mut out, i + 1, end);
                i = end + 1;
            },
            b'r' if !follows_ident(bytes, i) => {
                match raw_string(bytes, i) {
                    Some((open, close, after)) => {
                        blank(&mut out, open, close);
                        i = after;
                    },
                    None => i += 1,
                }
            },
            b'\'' => {
                match char_literal_end(bytes, i) {
                    Some(end) => {
                        blank(&mut out, i + 1, end);
                        i = end + 1;
                    },
                    // a lifetime or a label
                    None => i += 1,
                }
            },
            _ => i += 1,
        }
    }
    // every blanked run starts and ends on a character boundary and is filled with ASCII
    String::from_utf8(out).unwrap_or_default()
}

fn blank(out: &mut [u8], from: usize, to: usize) {
    let to = to.min(out.len());
    for b in out.iter_mut().take(to).skip(from) {
        if *b != b'\n' {
            *b = b' ';
        }
    }
}

fn line_end(bytes: &[u8], from: usize) -> usize {
    bytes[from ..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |n| from + n)
}

/// The offset past a block comment that opens at `from`. Block comments nest.
fn block_comment_end(bytes: &[u8], from: usize) -> usize {
    let mut depth = 0usize;
    let mut i = from;
    while i < bytes.len() {
        if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    bytes.len()
}

/// The offset of the quote that closes a string whose inside starts at `from`.
fn string_end(bytes: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return i,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// Whether the byte before `at` is part of an identifier, so an `r` at `at` ends a name and opens no
/// raw string.
fn follows_ident(bytes: &[u8], at: usize) -> bool {
    let ident = |i: usize| bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_';
    // `br"..."` is a raw byte string: the `b` before the `r` is its prefix and no name
    let byte_prefix = at > 0 && bytes[at - 1] == b'b' && (at == 1 || !ident(at - 2));
    at > 0 && ident(at - 1) && !byte_prefix
}

/// A raw string that opens at the `r` at `at`: where its inside starts, where it ends, and the offset
/// past its closing hashes.
fn raw_string(bytes: &[u8], at: usize) -> Option<(usize, usize, usize)> {
    let mut i = at + 1;
    let mut hashes = 0usize;
    while bytes.get(i) == Some(&b'#') {
        hashes += 1;
        i += 1;
    }
    if bytes.get(i) != Some(&b'"') {
        return None;
    }
    let open = i + 1;
    let mut j = open;
    while j < bytes.len() {
        if bytes[j] == b'"'
            && bytes[j + 1 ..]
                .iter()
                .take(hashes)
                .filter(|&&b| b == b'#')
                .count()
                == hashes
        {
            return Some((open, j, j + 1 + hashes));
        }
        j += 1;
    }
    Some((open, bytes.len(), bytes.len()))
}

/// The offset of the quote that closes a character literal opening at `at`, or nothing where the
/// quote opens a lifetime or a label.
fn char_literal_end(bytes: &[u8], at: usize) -> Option<usize> {
    match bytes.get(at + 1)? {
        b'\\' => {
            let mut i = at + 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                if bytes[i] == b'\'' && i > at + 2 {
                    return Some(i);
                }
                i += 1;
            }
            None
        },
        _ => {
            // one character of one to four bytes, then the closing quote
            (2 ..= 5)
                .map(|n| at + n)
                .find(|&i| bytes.get(i) == Some(&b'\''))
                .filter(|&i| {
                    std::str::from_utf8(&bytes[at + 1 .. i]).is_ok_and(|s| s.chars().count() == 1)
                })
        },
    }
}
