//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

use super::{ROUTES, STAMPED_BY, ticket};

#[test]
fn the_ticket_is_the_word_in_backticks_after_ticket() {
    assert_eq!(ticket("two experts agreed, ticket `abc123def456`"), Some("abc123def456"));
    assert_eq!(ticket("ticket `a_slug_id`, then ticket `another`"), Some("a_slug_id"));
    assert_eq!(ticket("the tickets list, and ticket ` spaced `"), Some("spaced"));
}

#[test]
fn a_stamp_naming_no_ticket_in_that_form_names_none() {
    // THE CONTROL for the test above: each of these is near the form and falls short of it.
    for stamp in [
        "two experts agreed",
        "ticket abc123",
        "ticket ``",
        "ticket `two words`",
        "ticket `unclosed",
        "tickets `abc123`",
    ] {
        assert_eq!(ticket(stamp), None, "{stamp}");
    }
}

#[test]
fn the_routes_are_the_values_the_pack_schema_declares_for_stamped_by() {
    let schema = include_str!("../../../registry/namespaces.toml");
    let field = schema
        .split("[[registry.namespace.field]]")
        .find(|f| f.contains(&format!("name = \"{STAMPED_BY}\"")))
        .expect("the pack schema declares `stamped_by`");
    let values = field
        .lines()
        .find_map(|l| l.trim().strip_prefix("values = "))
        .expect("`stamped_by` declares its values");
    let declared: Vec<&str> = values
        .trim_matches(['[', ']'])
        .split(',')
        .map(|v| v.trim().trim_matches('"'))
        .collect();
    assert_eq!(declared, ROUTES);
}
