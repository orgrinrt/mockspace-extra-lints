//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The tickets of the registry and the rules a move is refused by, read from the texts of the
//! registry's files. No file access and nothing of mockspace, so the tickets tool, its makers and the
//! console's page build this one crate and judge a move alike. The design is this crate's
//! `DESIGN.md.tmpl` and `mock/WORKFLOW.md.tmpl`, "Work items".

pub mod choice;
pub mod contracts;
pub mod gives;
pub mod heirs;
pub mod model;
pub mod options;
pub mod rules;
pub mod stale;
pub mod tracks;

pub use heirs::{Heirs, Repointed};
pub use model::{Fields, Kind, Move, State, Store, Ticket};
pub use rules::Finding;
pub use stale::Tree;
