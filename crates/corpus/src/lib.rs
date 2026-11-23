//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The repository's index as the tools ask it: which files feed it, where a checkout keeps it, the
//! words a question is asked in, and the maintainer's captured words.
//!
//! The registry is a few hundred rows. The research notes, the round transcripts and the
//! maintainer's captured words hold the argument, the evidence and most of the answers, and a tool
//! reading only the rows answers from a fraction of the material, confidently, and is believed. Every
//! tool that asks what the repository holds asks the one index this crate opens, which `search`
//! answers from and nothing builds in a question's path.

pub mod day;
pub mod maintainer;
pub mod repo;
pub mod terms;

pub use maintainer::Maintainer;
pub use terms::{Query, Source};
