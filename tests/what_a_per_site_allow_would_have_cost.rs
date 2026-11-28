//! What reading a foreign trait's signatures at the trait's owner saves a
//! crate that implements contracts it does not own, counted on one such crate:
//! a reader of layered manifests whose store implements another crate's store
//! contract, whose key and item walks are `Iterator`s, and whose view is read
//! into a tool's type through `serde`'s `Deserializer`, `MapAccess` and
//! `SeqAccess`. Each impl is written as such a crate writes it, one signature
//! a line, and read twice: as the impls of foreign traits they are, and as
//! inherent impls, which is how the lints read them with no exemption. Each
//! line the second reading reports is a line that would carry its own tracked
//! `lint:allow` were there no exemption.

use std::collections::BTreeSet;

use mockspace_extra_lints::lints::arvo_types_only::ArvoTypesOnly;
use mockspace_extra_lints::lints::no_bare_numeric::NoBareNumeric;
use mockspace_extra_lints::lints::no_bare_option::NoBareOption;
use mockspace_extra_lints::lints::no_bare_result::NoBareResult;
use mockspace_extra_lints::lints::no_bare_static_str::NoBareStaticStr;
use mockspace_extra_lints::lints::no_bare_string::NoBareString;
use mockspace_lint_rules::testkit::LintFixture;
use mockspace_lint_rules::CrateLint;

/// A crate of the stack's vocabulary, which every stack lint holds.
const VOCABULARY_MANIFEST: &str = "[package]\nname = \"notko-fixture\"\n";

const STORE: &str = "impl contracts::Store for Toml {
    const NAME: &'static str = \"toml\";
    const EXTENSION: &'static str = \"toml\";
    type Document<'t> = Doc<'t>;
    type Keys<'d> = KeyWalk<'d>;
    type Items<'d> = ItemWalk<'d>;
    fn parse<'t>(text: &'t str) -> Outcome<Self::Document<'t>, BadDocument> { read(text) }
    fn get<'d>(doc: &'d Self::Document<'_>, key: &str) -> Maybe<Literal<'d>> { doc.at(key) }
    fn keys<'d>(doc: &'d Self::Document<'_>) -> Self::Keys<'d> { doc.walk() }
    fn items<'d>(list: &'d str) -> Self::Items<'d> { ItemWalk::over(list) }
    fn shape(doc: &Self::Document<'_>, key: &str) -> Maybe<Shape> { doc.shape(key) }
    fn set(text: &str, key: &str, value: Rendered<'_>, into: &mut impl fmt::Write) -> fmt::Result { edit(text, key, value, into) }
}
impl<'d> Iterator for KeyWalk<'d> {
    type Item = &'d str;
    fn next(&mut self) -> Option<&'d str> { self.step() }
}
impl<'d> Iterator for ItemWalk<'d> {
    type Item = Literal<'d>;
    fn next(&mut self) -> Option<Literal<'d>> { self.step() }
}
";

/// The deserializer's required methods, each as `serde` declares it.
const DESERIALIZER: &str = "impl<'de> serde::Deserializer<'de> for View<'de> {
    type Error = BadRead;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_tuple_struct<V: Visitor<'de>>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_struct<V: Visitor<'de>>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_enum<V: Visitor<'de>>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.any(visitor) }
}
impl<'de> serde::de::MapAccess<'de> for Table<'de> {
    type Error = BadRead;
    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> { self.key(seed) }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Self::Error> { self.value(seed) }
}
impl<'de> serde::de::SeqAccess<'de> for Run<'de> {
    type Error = BadRead;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> { self.element(seed) }
}
";

/// The lines of `src/lib.rs` every bare-type lint reports for `lib`, each
/// once however many lints report it.
fn reported_lines(lib: &str) -> BTreeSet<usize> {
    let lints: [&dyn CrateLint; 6] = [
        &NoBareResult,
        &NoBareOption,
        &NoBareString,
        &NoBareStaticStr,
        &NoBareNumeric,
        &ArvoTypesOnly,
    ];
    let fixture = LintFixture::new(lib)
        .with_doc("Cargo.toml", VOCABULARY_MANIFEST)
        .with_workspace_root(std::path::Path::new("/tmp"));
    lints
        .iter()
        .flat_map(|lint| lint.check(&fixture.ctx()))
        .filter(|e| e.path.as_deref().is_none_or(|p| p.ends_with("lib.rs")))
        .map(|e| e.line)
        .collect()
}

/// `impl <Trait> for <Type>` made `impl <Type>` on every line, so nothing in
/// it is a trait's.
fn inherent(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            match (line.strip_prefix("impl"), line.find(" for ")) {
                (Some(_), Some(at)) => {
                    let generics = line[4 ..].split_whitespace().next().filter(|g| g.starts_with('<'));
                    format!("impl{} {}", generics.map_or(String::new(), |g| format!("{g}")), &line[at + 5 ..])
                },
                _ => line.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_per_site_allow_would_have_cost_the_reader_a_marker_on_every_signature() {
    let lib = format!("{STORE}{DESERIALIZER}");
    // read as what they are, the impls' signatures are the traits' and pass;
    // what is left is the one item the implementer chose, the key walk's
    // `Item`, which is no signature and would carry its allow either way
    let item = lib.lines().position(|l| l.contains("type Item = &'d str;")).unwrap() + 1;
    assert_eq!(reported_lines(&lib), BTreeSet::from([item]));
    // read with no exemption, each signature with a bare type the lints see is
    // a line that would carry its own tracked allow. `no-bare-result` sees the
    // deserializer's `Result` only on the five lines that hold a second quote
    // after the lifetime's (ticket
    // mockspace_extra_lints_no_bare_result_misses_a_result_after_a); every one
    // of its 33 signatures holds one, so the cost is 9 + 33 = 42 lines once it
    // sees them all
    let sites = |source: &str| {
        let mut lines = reported_lines(&inherent(source));
        lines.remove(&(source.lines().position(|l| l.contains("type Item = &'d str;")).map_or(0, |i| i + 1)));
        lines.len()
    };
    let (store, deserializer, both) = (sites(STORE), sites(DESERIALIZER), sites(&lib));
    assert_eq!(
        (store, deserializer, both),
        (9, 5, 14),
        "the store's lines, the deserializer's lines, and both"
    );
}
