use super::*;

#[test]
fn the_names_are_read_from_the_root_of_the_config() {
    let p = Project::from_config(
        "# a comment\nproject_name = \"arvo\"\ncrate_prefix = \"arvo\"\n\n[lints.x]\nproject_name = \"no\"\n",
    );
    assert_eq!((p.name.as_str(), p.prefix.as_str()), ("arvo", "arvo"));
}

#[test]
fn the_prefix_defaults_to_the_name_and_the_name_to_project() {
    assert_eq!(Project::from_config("project_name = \"kaski\"\n").prefix, "kaski");
    assert_eq!(Project::from_config("").name, "project");
}

#[test]
fn a_key_inside_a_table_is_not_the_root_key() {
    assert_eq!(Project::from_config("[a]\ncrate_prefix = \"x\"\n").prefix, "project");
}

#[test]
fn a_crate_is_unprefixed_only_with_its_dash() {
    let p = Project::from_config("crate_prefix = \"kaski\"\n");
    assert_eq!(p.unprefixed("kaski-sim"), Some("sim"));
    assert_eq!(p.unprefixed("kaskisim"), None);
    assert_eq!(p.unprefixed("arvo-sim"), None);
}
