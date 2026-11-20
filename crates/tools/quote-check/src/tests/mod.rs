//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The comparison, the locator and the coverage, each against a source held in memory, and every
//! verdict beside the near case that must not reach it.

mod compare;
mod coverage;
mod locator;
mod options;

pub(crate) use crate::source::Memory;
