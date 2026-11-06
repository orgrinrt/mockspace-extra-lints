use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::slang::read;

fn at(crate_name: &str, file: &str, text: &str) -> SlangSource {
    SlangSource {
        rel:        format!("mock/crates/{crate_name}/shaders/{file}.slang"),
        crate_name: crate_name.to_string(),
        text:       text.to_string(),
        file:       read(text),
    }
}

fn deps(edges: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
    edges
        .iter()
        .map(|(c, d)| (c.to_string(), d.iter().map(|x| x.to_string()).collect()))
        .collect()
}

/// `top` depends on `mid`, `mid` on `base`, and `lone` on nothing.
fn graph() -> Graph {
    Graph::build(
        &[
            at("base", "lattice", "module lattice;\npublic float hash(float a) { return a; }\n"),
            at("mid", "march", "module march;\nimport lattice;\npublic float step(float a) { return a; }\n"),
            at("top", "frame", "import march;\nimport lattice;\nimport own;\n[shader(\"compute\")]\nvoid cs(uint3 id) {}\n"),
            at("top", "own", "module own;\n"),
            at("lone", "island", "module island;\n"),
        ],
        deps(&[("top", &["mid"]), ("mid", &["base"])]),
    )
}

fn named(g: &Graph, name: &str) -> usize {
    g.modules.iter().position(|m| m.name == name).unwrap()
}

#[test]
fn a_module_is_named_by_its_module_line_and_a_file_without_one_by_its_stem() {
    let g = graph();
    assert!(g.modules.iter().any(|m| m.name == "frame"), "{:?}", g.modules);
    assert_eq!(g.modules[named(&g, "lattice")].exports, ["hash"]);
    assert_eq!(g.modules[named(&g, "frame")].entries, [("cs".to_string(), "compute".to_string())]);
}

#[test]
fn an_import_leads_to_an_own_module_a_reached_one_or_nowhere() {
    let g = graph();
    assert_eq!(g.resolve("top", "own"), Resolution::Own(named(&g, "own")));
    assert_eq!(g.resolve("top", "march"), Resolution::Dependency(named(&g, "march")));
    assert_eq!(g.resolve("top", "lattice"), Resolution::Dependency(named(&g, "lattice")));
    assert_eq!(g.resolve("top", "nothing"), Resolution::Missing);
}

#[test]
fn a_module_of_a_crate_the_importer_does_not_reach_is_unreached() {
    let g = graph();
    assert_eq!(g.resolve("top", "island"), Resolution::Unreached(named(&g, "island")));
    assert_eq!(g.resolve("base", "march"), Resolution::Unreached(named(&g, "march")));
}

#[test]
fn the_search_path_is_the_own_crate_then_every_crate_it_reaches_with_shaders() {
    let g = graph();
    assert_eq!(g.search_path("top"), ["top", "base", "mid"]);
    assert_eq!(g.search_path("base"), ["base"]);
    assert!(g.search_path("nowhere").is_empty());
}

#[test]
fn a_dependency_cycle_ends() {
    let g = Graph::build(&[], deps(&[("a", &["b"]), ("b", &["a"])]));
    assert_eq!(g.reach("a"), BTreeSet::from(["b".to_string()]));
}

#[test]
fn the_importers_of_a_module_are_those_whose_imports_lead_to_it() {
    let g = graph();
    let lattice = named(&g, "lattice");
    let who: Vec<&str> = g.importers(lattice).into_iter().map(|i| g.modules[i].name.as_str()).collect();
    assert_eq!(who, ["march", "frame"]);
}

#[test]
fn a_probe_is_not_a_module() {
    let mut p = at("top", "x", "module probe;\n");
    p.rel = "mock/crates/top/src/tests/probe/x.slang".to_string();
    assert!(Graph::build(&[p], BTreeMap::new()).modules.is_empty());
}
