//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The struck rows the two struck-row test files plant, and what the report
//! says of them.
//!
//! Apart from `coverage_harness` because the corpus file uses none of it, and a
//! module compiled into a test target that does not use it warns.

use mockspace_lint_rules::RegistryView;

use crate::coverage_harness::{DEMAND, view};

/// What the report says of a row reaching nothing whose answers were struck.
pub const WITHDRAWN: &str = "were answered by rows since struck";

/// What the report says of a row answered by nothing and carrying a
/// precondition.
pub const STUCK: &str = "carry an established precondition";

/// A proposal naming the demand row, carrying `retired` with the value given.
pub fn struck_proposal(by: &'static str) -> RegistryView {
    view(&[
        DEMAND,
        ("proposal::a_claim", &[
            ("obligation", "the_thing"),
            ("retired", by),
        ]),
    ])
}

/// A ratified ruling stamping a proposal that names the demand row, with the
/// ruling's `retired` and then the proposal's. Blank is absent.
pub fn stamp(ruling: &'static str, proposal: &'static str) -> RegistryView {
    view(&[
        DEMAND,
        ("ruling::he_said_so", &[
            ("rung", "ratified"),
            ("ratifies", "a_claim"),
            ("retired", ruling),
        ]),
        ("proposal::a_claim", &[
            ("obligation", "the_thing"),
            ("retired", proposal),
        ]),
    ])
}
