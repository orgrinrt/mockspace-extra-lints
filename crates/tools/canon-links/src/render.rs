//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The views as text.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use canon::Tenet;

use crate::THIN_KEYWORDS;
use crate::measure::{Counts, Totals, counts, totals};
use crate::probes::{Outcome, Via};

fn totals_block(out: &mut String, t: &Totals) {
    let mean = if t.rows == 0 { 0.0 } else { t.keywords as f64 / t.rows as f64 };
    let _ = writeln!(
        out,
        "  tenets            {:>4}  ({} binding)",
        t.rows, t.binding
    );
    let _ = writeln!(
        out,
        "  keywords          {:>4}  ({mean:.2} a tenet)",
        t.keywords
    );
    let _ = writeln!(out, "  no keywords       {:>4}", t.no_keywords);
    let _ = writeln!(out, "  under {THIN_KEYWORDS} keywords   {:>4}", t.thin);
    let _ = writeln!(out, "  cites edges       {:>4}", t.cites_edges);
    let _ = writeln!(out, "  link edges        {:>4}", t.link_edges);
    let _ = writeln!(out, "  cite or relate none  {:>4}", t.no_outgoing);
    let _ = writeln!(out, "  cited or related by none  {:>4}", t.no_incoming);
    let _ = writeln!(out, "  joined to nothing {:>4}", t.islands);
}

/// The totals, the same per subject, and the rows joined to nothing.
pub fn totals_view(all: &[Tenet<'_>]) -> String {
    let c = counts(all);
    let every: Vec<&Counts<'_>> = c.iter().collect();
    let mut out = String::from("CANON METADATA\n\n");
    totals_block(&mut out, &totals(&every, all));
    out.push_str("\nPER SUBJECT\n\n");
    let subjects: BTreeSet<&str> = c.iter().map(|r| r.subject).collect();
    let w = subjects.iter().map(|s| s.len()).max().unwrap_or(0);
    let _ = writeln!(
        out,
        "  {:<w$}  rows  kw/row  thin  no-out  no-in  islands",
        "subject"
    );
    for s in subjects {
        let rows: Vec<&Counts<'_>> = c.iter().filter(|r| r.subject == s).collect();
        let t = totals(&rows, all);
        let _ = writeln!(
            out,
            "  {s:<w$}  {:>4}  {:>6.2}  {:>4}  {:>6}  {:>5}  {:>7}",
            t.rows,
            t.keywords as f64 / t.rows.max(1) as f64,
            t.thin,
            t.no_outgoing,
            t.no_incoming,
            t.islands
        );
    }
    let lonely: Vec<&&Counts<'_>> = every.iter().filter(|r| r.binds && r.is_island()).collect();
    let _ = writeln!(out, "\nBINDING ROWS JOINED TO NOTHING ({})\n", lonely.len());
    for r in lonely {
        let _ = writeln!(out, "  {}", r.id);
    }
    out
}

/// One line a tenet.
pub fn rows_view(all: &[Tenet<'_>]) -> String {
    let c = counts(all);
    let w = c.iter().map(|r| r.id.len()).max().unwrap_or(0);
    let mut out = format!("  {:<w$}  kw  out   in  subject\n", "tenet");
    for r in &c {
        let _ = writeln!(
            out,
            "  {:<w$}  {:>2}  {:>3}  {:>3}  {}{}",
            r.id,
            r.keywords,
            r.outgoing,
            r.incoming,
            r.subject,
            if r.binds { "" } else { "  (superseded)" }
        );
    }
    out
}

/// Every probe with what it found, then the totals.
pub fn probes_view(outcomes: &[Outcome]) -> String {
    let mut out = String::from("PROBES\n\n");
    let (mut want, mut got, mut whole) = (0, 0, 0);
    for o in outcomes {
        let via = match &o.probe.via {
            Via::Find(w) => format!("find {w}"),
            Via::Related(id) => format!("related {id}"),
        };
        let total = o.found.len() + o.missing.len();
        want += total;
        got += o.found.len();
        whole += usize::from(o.missing.is_empty() && o.unknown.is_empty());
        let _ = writeln!(
            out,
            "  {}: {via}: {} of {total} expected, {} returned",
            o.probe.ask,
            o.found.len(),
            o.returned
        );
        for m in &o.missing {
            let _ = writeln!(out, "      missing  {m}");
        }
        for u in &o.unknown {
            let _ = writeln!(out, "      NOT A TENET  {u}");
        }
    }
    let _ = writeln!(
        out,
        "\n  {} probes, {whole} answered whole, {got} of {want} expected tenets returned",
        outcomes.len()
    );
    out
}
