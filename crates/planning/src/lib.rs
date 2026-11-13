//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The planning rows of the registry and the rules a schema cannot state over them, read from the
//! texts of the registry's files. No file access and nothing of mockspace, and the tickets crate
//! names nothing of this one. The design is this crate's `DESIGN.md`.

pub mod derive;
pub mod plan;
pub mod roadmap;
pub mod rules;
pub mod time;
pub mod write;

pub use derive::{Class, Counts, Facts, MilestoneReport, SpanReport, TicketFacts};
pub use plan::{Plan, Row};
pub use roadmap::Roadmap;
pub use write::{Home, NewRow};

/// The planning namespaces a registry declares, each a key `mockspace.toml` holds.
pub const NAMESPACES: [&str; 13] = [
    "milestone",
    "milestone_merge",
    "milestone_scope",
    "plan_span",
    "member",
    "window",
    "plan_slot",
    "change",
    "landing",
    "review",
    "repo",
    "plan_kind_role",
    "plan_state_role",
];
