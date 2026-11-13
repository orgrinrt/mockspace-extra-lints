//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------
//! A move's id. A ticket's id is a slug of its title, which `kaski-tickets` makes.

use kaski_tickets::rules::slugify;

use crate::time::now_secs;

/// Four hex digits that differ between two calls in the same second: the clock's nanoseconds and the
/// process, mixed.
fn nonce(salt: u32) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::from(d.subsec_nanos()));
    let mixed = (nanos ^ (u64::from(std::process::id()) << 7) ^ now_secs() ^ u64::from(salt))
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    format!("{:04x}", (mixed >> 40) & 0xFFFF)
}

/// A move's id: `m`, the time's digits, the author and four hex digits. `attempt` changes the last
/// part for the retry after a collision.
pub fn move_id(at: &str, by: &str, attempt: u32) -> String {
    let digits: String = at.chars().filter(char::is_ascii_digit).collect();
    let who = slugify(by);
    format!("m{digits}_{who}_{}", nonce(attempt))
}
