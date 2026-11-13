//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The lints over a registry's tickets, tenets and subjects, and the readings of the tree they share
//! with the tools.
//!
//! Each lint file declares its lint by a `repo_lint` function, the shape a repository's own
//! `<mock>/lints/` file has, and [`collect`] hands every one to the pack. The files that declare no
//! lint are the one reading of their concern, public so the tools and a consumer's own lints read by
//! them rather than by a copy: what a marker is (`fixme_markers`), which files are walked
//! (`tree_walk`, `canon_tree`), the tenets (`canon_rows`), the words a tenet retires
//! (`retired_words`), the waivers (`ratchets`), what a merge carries (`merge_carried`), the lines a
//! change touched (`touched_lines`) and a lint's table in `mockspace.toml` (`gate_config`).

use mockspace::LintPack;

pub mod a_live_row_names_no_replaced_row_without_lineage;
pub mod a_relation_adds_what_the_words_do_not;
pub mod a_retired_word_stays_out;
pub mod a_tenet_cites_what_its_words_name;
pub mod a_tenet_names_a_subject;
pub mod a_ticket_graph_holds;
pub mod canon_rows;
pub mod canon_tree;
pub mod every_fixme_names_a_ticket;
pub mod fixme_markers;
pub mod gate_config;
pub mod merge_carried;
pub mod project;
pub mod ratchets;
pub mod retired_words;
pub mod touched_lines;
pub mod tree_walk;

/// Hand every lint of this crate to the pack.
pub fn collect(pack: &mut LintPack) {
    pack.repo_lints.extend([
        a_live_row_names_no_replaced_row_without_lineage::repo_lint(),
        a_relation_adds_what_the_words_do_not::repo_lint(),
        a_retired_word_stays_out::repo_lint(),
        a_tenet_cites_what_its_words_name::repo_lint(),
        a_tenet_names_a_subject::repo_lint(),
        a_ticket_graph_holds::repo_lint(),
        every_fixme_names_a_ticket::repo_lint(),
    ]);
}
