//! How close two declarations are, and which pairs of a set are worth reading.
//!
//! Nothing here reads a file: it is handed the declarations `declared_types` produced and returns
//! the pairs. A member is matched in two passes. The first pairs members that are the same whole, a
//! name over a body. The second pairs what is left by body alone, which is a renamed field: the same
//! type under another name. The closeness is the share of the union that matched, a renamed member
//! counting half, since it is the same thing under a word nobody chose twice.

use mockspace_lint_rules::{DeclaredMember, DeclaredType};

/// How two declarations of one keyword line up, member by member.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Close {
    /// Members that are the same name over the same body.
    pub shared:          Vec<String>,
    /// Members matched by body alone, as `(name in the first, name in the second, body)`.
    pub renamed:         Vec<(String, String, String)>,
    /// Members only the first has, as written whole.
    pub only_first:      Vec<String>,
    /// Members only the second has.
    pub only_second:     Vec<String>,
    /// Whether the generics, bounds included, differ.
    pub generics_differ: bool,
    /// Whether what the declaration keeps beside its members differs: supertrait bounds, the kind of
    /// body, the order of a numbered enum.
    pub frame:           Option<(String, String)>,
    /// Set when the two are of different keywords, where nothing lines up.
    pub kinds_differ:    bool,
}

impl Close {
    /// The share of the members that line up, as a whole percent. Two declarations with no members
    /// between them line up in nothing.
    #[must_use]
    pub fn percent(&self) -> usize {
        let union =
            self.shared.len() + self.renamed.len() + self.only_first.len() + self.only_second.len();
        if union == 0 {
            return 0;
        }
        (2 * self.shared.len() + self.renamed.len()) * 100 / (2 * union)
    }

    /// How many members line up at all, whether by name or by body.
    #[must_use]
    pub fn lined_up(&self) -> usize {
        self.shared.len() + self.renamed.len()
    }
}

/// Lines two declarations up. `None` when they are of different keywords, which is never one concept
/// copied.
#[must_use]
pub fn close(a: &DeclaredType, b: &DeclaredType) -> Option<Close> {
    if a.kind != b.kind {
        return None;
    }
    let mut left: Vec<&DeclaredMember> = a.parts.members.iter().collect();
    let mut right: Vec<&DeclaredMember> = b.parts.members.iter().collect();
    let mut out = Close {
        generics_differ: a.parts.generics != b.parts.generics,
        frame: (a.parts.frame != b.parts.frame)
            .then(|| (a.parts.frame.clone(), b.parts.frame.clone())),
        ..Close::default()
    };
    take_pairs(
        &mut left,
        &mut right,
        |l, r| l.whole() == r.whole(),
        |l, _| out.shared.push(l.name.clone()),
    );
    let mut renamed = Vec::new();
    take_pairs(
        &mut left,
        &mut right,
        |l, r| !l.body.is_empty() && l.body == r.body,
        |l, r| renamed.push((l.name.clone(), r.name.clone(), l.body.clone())),
    );
    out.renamed = renamed;
    out.only_first = left.iter().map(|m| m.whole()).collect();
    out.only_second = right.iter().map(|m| m.whole()).collect();
    Some(out)
}

/// Removes from both lists every pair `same` accepts, each member used once, handing `found` the pair.
fn take_pairs<'m>(
    left: &mut Vec<&'m DeclaredMember>,
    right: &mut Vec<&'m DeclaredMember>,
    same: impl Fn(&DeclaredMember, &DeclaredMember) -> bool,
    mut found: impl FnMut(&DeclaredMember, &DeclaredMember),
) {
    let mut kept = Vec::new();
    for l in left.drain(..) {
        match right.iter().position(|r| same(l, r)) {
            Some(at) => found(l, right.remove(at)),
            None => kept.push(l),
        }
    }
    *left = kept;
}

/// What the person asked of the comparison.
#[derive(Debug, Clone)]
pub struct Options {
    /// A near-copy is listed from this closeness, a whole percent. A knob: nobody has settled where
    /// the line between near and unrelated falls, and the ranking is the answer, not the line.
    pub min_percent: usize,
    /// Only types that are `pub` without restriction are read.
    pub pub_only:    bool,
}

/// One pair of types in two crates, and how they line up.
pub struct Pair<'t> {
    pub first:  &'t DeclaredType,
    pub second: &'t DeclaredType,
    pub close:  Close,
}

/// The two lists, each ranked by closeness, closest first.
#[derive(Default)]
pub struct Found<'t> {
    /// Different names, close declarations.
    pub near_copies: Vec<Pair<'t>>,
    /// One name, different declarations.
    pub same_named:  Vec<Pair<'t>>,
}

/// Every pair of types in different crates where `in_scope` holds for at least one side, and the
/// pair is either one name over two declarations or two names over close ones. One name over one
/// declaration is what `single-source` gates on and is left to it.
#[must_use]
pub fn compare<'t>(
    types: &'t [DeclaredType],
    in_scope: &dyn Fn(&DeclaredType) -> bool,
    options: &Options,
) -> Found<'t> {
    let read: Vec<&DeclaredType> = types
        .iter()
        .filter(|t| !options.pub_only || t.fully_pub())
        .collect();
    let mut found = Found::default();
    for (i, first) in read.iter().enumerate() {
        for second in &read[i + 1 ..] {
            if first.crate_name == second.crate_name || !(in_scope(first) || in_scope(second)) {
                continue;
            }
            let same_name = first.name == second.name;
            if same_name && first.shape == second.shape {
                continue;
            }
            let Some(lined) = close(first, second) else {
                if same_name {
                    found.same_named.push(Pair {
                        first,
                        second,
                        close: Close {
                            kinds_differ: true,
                            ..Close::default()
                        },
                    });
                }
                continue;
            };
            let pair = Pair {
                first,
                second,
                close: lined,
            };
            if same_name {
                found.same_named.push(pair);
            } else if pair.close.lined_up() >= 2 && pair.close.percent() >= options.min_percent {
                found.near_copies.push(pair);
            }
        }
    }
    for list in [&mut found.near_copies, &mut found.same_named] {
        list.sort_by(|a, b| {
            b.close
                .percent()
                .cmp(&a.close.percent())
                .then_with(|| key(a).cmp(&key(b)))
        });
    }
    found
}

fn key<'t>(p: &Pair<'t>) -> (&'t str, &'t str, &'t str, &'t str) {
    (
        &p.first.crate_name,
        &p.first.name,
        &p.second.crate_name,
        &p.second.name,
    )
}
