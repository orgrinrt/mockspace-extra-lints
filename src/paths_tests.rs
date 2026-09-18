use super::*;

fn generics_of(src: &str) -> Vec<String> {
    let tree = parse(src).unwrap();
    let item = tree.root_node().named_child(0).unwrap();
    let mut out = BTreeSet::new();
    generic_names(item, src, &mut out);
    out.into_iter().collect()
}

#[test]
fn every_shape_of_type_parameter_is_collected() {
    for (src, want) in [
        ("fn f<T>() {}", &["T"][..]),
        ("fn f<T: Clone>() {}", &["T"]),
        ("fn f<T: Clone + Send, U>() {}", &["T", "U"]),
        ("fn f<T>() where T: Clone {}", &["T"]),
        ("struct S<T = u8>(T);", &["T"]),
        ("struct S<T: Clone = u8>(T);", &["T"]),
        ("enum E<K, S: Store> { A(K, S) }", &["K", "S"]),
        ("trait Tr<T> {}", &["T"]),
        ("impl<St: Store> Held<St> {}", &["St"]),
    ] {
        assert_eq!(generics_of(src), want, "{src}");
    }
}

#[test]
fn a_lifetime_a_const_and_no_parameters_collect_no_type_name() {
    for src in [
        "fn f() {}",
        "fn f<'a>(x: &'a u8) {}",
        "fn f<const N: usize>() {}",
        "struct S;",
    ] {
        assert_eq!(generics_of(src), Vec::<String>::new(), "{src}");
    }
    // Beside a type parameter, only the type parameter.
    assert_eq!(generics_of("fn f<'a, T, const N: usize>(x: &'a T) {}"), ["T"]);
}
