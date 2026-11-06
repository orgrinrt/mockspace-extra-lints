//! The modules of the shipped shaders and the imports between them.
//!
//! A shader module is a `.slang` file in a crate's `shaders/` directory, named by the `module` line it
//! opens with, or by its file name where it has none. A crate may import its own modules and the modules of
//! the crates it depends on, directly or through another, and nothing else, so what a crate's compile
//! searches is the one list this graph gives it: the crate's own `shaders/` and then those of the crates it
//! reaches. The graph knows nothing of cargo. It is given, for each crate, the crates it depends on, so
//! a project reads its own manifests and the graph stays the one place the search path is made.

use std::collections::{BTreeMap, BTreeSet};

use super::{Import, SlangSource};

/// One module of a crate's shaders.
#[derive(Debug, Clone)]
pub struct Module {
    pub crate_name: String,
    /// The path of the file from the repository root.
    pub rel:        String,
    /// The name it is imported by.
    pub name:       String,
    pub imports:    Vec<Import>,
    /// What it offers other modules.
    pub exports:    Vec<String>,
    /// The entries it declares, as `(function, stage)`.
    pub entries:    Vec<(String, String)>,
}

/// Where an import leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// A module of the importing crate.
    Own(usize),
    /// A module of a crate the importing crate reaches through its dependencies.
    Dependency(usize),
    /// A module of a crate the importing crate does not reach: it exists, but the edge does not.
    Unreached(usize),
    /// No module of that name in any crate.
    Missing,
}

/// The shipped modules, and the crates each crate depends on.
#[derive(Debug, Clone, Default)]
pub struct Graph {
    pub modules: Vec<Module>,
    /// For each crate, the crates it depends on directly.
    deps:        BTreeMap<String, BTreeSet<String>>,
}

impl Graph {
    /// The graph of the shipped sources, given each crate's direct dependencies.
    pub fn build(sources: &[SlangSource], deps: BTreeMap<String, BTreeSet<String>>) -> Self {
        let modules = sources
            .iter()
            .filter(|s| s.shipped())
            .map(|s| Module {
                crate_name: s.crate_name.clone(),
                rel:        s.rel.clone(),
                name:       s.file.module.as_ref().map_or_else(|| s.stem().to_string(), |m| m.0.clone()),
                imports:    s.file.imports.clone(),
                exports:    s.file.exports().map(|i| i.name.clone()).collect(),
                entries:    s.file.entries().map(|(i, stage)| (i.name.clone(), stage.to_string())).collect(),
            })
            .collect();
        Self { modules, deps }
    }

    /// The crates `crate_name` reaches: its dependencies, and theirs, and so on, without itself.
    pub fn reach(&self, crate_name: &str) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut open = vec![crate_name.to_string()];
        while let Some(next) = open.pop() {
            for d in self.deps.get(&next).into_iter().flatten() {
                if seen.insert(d.clone()) {
                    open.push(d.clone());
                }
            }
        }
        seen.remove(crate_name);
        seen
    }

    /// Where the import `name` written in a module of `from` leads.
    pub fn resolve(&self, from: &str, name: &str) -> Resolution {
        let reach = self.reach(from);
        let hits: Vec<(usize, &Module)> = self.modules.iter().enumerate().filter(|(_, m)| m.name == name).collect();
        if let Some((i, _)) = hits.iter().find(|(_, m)| m.crate_name == from) {
            return Resolution::Own(*i);
        }
        if let Some((i, _)) = hits.iter().find(|(_, m)| reach.contains(&m.crate_name)) {
            return Resolution::Dependency(*i);
        }
        hits.first().map_or(Resolution::Missing, |(i, _)| Resolution::Unreached(*i))
    }

    /// The directories, as crate names, a compile of one of `crate_name`'s shaders searches: its own
    /// first and then every crate it reaches that has shaders, in name order.
    pub fn search_path(&self, crate_name: &str) -> Vec<String> {
        let has_shaders = |c: &str| self.modules.iter().any(|m| m.crate_name == c);
        let mut out = Vec::new();
        if has_shaders(crate_name) {
            out.push(crate_name.to_string());
        }
        out.extend(self.reach(crate_name).into_iter().filter(|c| has_shaders(c)));
        out
    }

    /// The modules whose imports lead to the module at `index`.
    pub fn importers(&self, index: usize) -> Vec<usize> {
        let target = &self.modules[index];
        self.modules
            .iter()
            .enumerate()
            .filter(|(i, m)| {
                *i != index
                    && m.imports.iter().any(|imp| {
                        imp.name == target.name
                            && matches!(
                                self.resolve(&m.crate_name, &imp.name),
                                Resolution::Own(t) | Resolution::Dependency(t) if t == index
                            )
                    })
            })
            .map(|(i, _)| i)
            .collect()
    }
}

#[cfg(test)]
mod tests;
