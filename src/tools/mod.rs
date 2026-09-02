//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Tools this pack ships, for every consumer rather than for one.
//!
//! # Why a tool lives here rather than in a repository
//!
//! A tool arrives through `lint-crates` on the same cdylib a lint does, and
//! `mock tools` enumerates it identically whichever side it came from. So the
//! question of where one belongs is the question a lint already answers: a
//! check that only one repository will ever want stays in that repository's
//! `<mock>/tools/`, and a check several want is one implementation here rather
//! than one per consumer that drift apart.
//!
//! Two corpora had grown their own answers to the same questions with no
//! cross-citation. The provenance of a ruling was audited by two tools with
//! different names in different repositories, and coverage of a namespace by
//! two more. Neither pair knew about the other, and a fix to one was a fix to
//! one.
//!
//! # What generalising one actually took
//!
//! Less than it looks, and the reason is worth stating because it is what makes
//! the rest of the port tractable. A tool is already written against
//! [`mockspace_lint_rules::tool::ToolContext`], which hands it a
//! [`mockspace_lint_rules::RegistryView`] rather than a path, so nothing in a
//! well-written one reaches for a repository's own shape. What couples a tool
//! to its origin is a handful of string constants naming namespaces and fields.
//!
//! **A field a corpus does not declare is the case to get right**, and the
//! honest handling is to treat its absence as "this corpus does not make that
//! distinction" rather than to assume a value. Assuming reads as working and is
//! wrong in one direction only, which is the direction nobody checks.

pub mod rulings_with_no_verbatim;
