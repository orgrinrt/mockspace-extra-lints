//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Counting what each tenet carries: keywords, and the relations it has in each direction.
//!
//! A relation is a `cites` or a tenet link in `related`, which the tools treat alike. Supersession is a different
//! kind of edge and is not counted here, so a row that only replaces another still reads as unjoined.

use std::collections::BTreeMap;

use canon::Tenet;

/// A row with fewer keywords than this is thin. A guess to tune: the canon's rows mostly carry three
/// or four, and a reader types one word of the several a concept goes by.
pub const THIN_KEYWORDS: usize = 4;

/// What one tenet carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counts<'a> {
    pub id:       &'a str,
    pub subject:  &'a str,
    pub binds:    bool,
    pub keywords: usize,
    /// Distinct tenets this one cites or links.
    pub outgoing: usize,
    /// Distinct tenets that cite or relate to this one.
    pub incoming: usize,
}

impl Counts<'_> {
    /// Joined to nothing in either direction.
    pub fn is_island(&self) -> bool {
        self.outgoing == 0 && self.incoming == 0
    }
}

/// Every tenet's counts, in id order.
pub fn counts<'a>(all: &[Tenet<'a>]) -> Vec<Counts<'a>> {
    let mut incoming: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for t in all {
        for n in t.cites.iter().chain(&t.links) {
            let from = incoming.entry(*n).or_default();
            if *n != t.id && !from.contains(&t.id) {
                from.push(t.id);
            }
        }
    }
    all.iter()
        .map(|t| {
            let mut out: Vec<&str> = Vec::new();
            for n in t.cites.iter().chain(&t.links) {
                if *n != t.id && !out.contains(n) {
                    out.push(n);
                }
            }
            Counts {
                id:       t.id,
                subject:  t.subject.unwrap_or("none"),
                binds:    t.binds(),
                keywords: t.keywords.len(),
                outgoing: out.len(),
                incoming: incoming.get(t.id).map_or(0, Vec::len),
            }
        })
        .collect()
}

/// The totals over a set of rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Totals {
    pub rows:        usize,
    pub binding:     usize,
    pub keywords:    usize,
    pub no_keywords: usize,
    pub thin:        usize,
    pub cites_edges: usize,
    pub link_edges:  usize,
    pub no_outgoing: usize,
    pub no_incoming: usize,
    pub islands:     usize,
}

/// The totals over `rows`, with the edge counts read from `all`.
pub fn totals(rows: &[&Counts<'_>], all: &[Tenet<'_>]) -> Totals {
    let ids: Vec<&str> = rows.iter().map(|c| c.id).collect();
    let mut t = Totals {
        rows: rows.len(),
        ..Totals::default()
    };
    for c in rows {
        t.binding += usize::from(c.binds);
        t.keywords += c.keywords;
        t.no_keywords += usize::from(c.keywords == 0);
        t.thin += usize::from(c.keywords < THIN_KEYWORDS);
        t.no_outgoing += usize::from(c.outgoing == 0);
        t.no_incoming += usize::from(c.incoming == 0);
        t.islands += usize::from(c.is_island());
    }
    for r in all.iter().filter(|r| ids.contains(&r.id)) {
        t.cites_edges += r.cites.len();
        t.link_edges += r.links.len();
    }
    t
}
