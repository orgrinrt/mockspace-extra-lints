//! `find` with the near words of the index: what they add, and that they add nothing the plain
//! answer already said.

use corpus::repo::Near;

use crate::Canon;
use crate::tests::{answered, view};

fn near_of(word: &str, synonym: bool) -> Near {
    Near {
        word: word.to_string(),
        synonym,
        rows: usize::from(synonym),
    }
}

fn reg() -> mockspace::RegistryView {
    view(&[
        ("subject::air", &[("what", "The air.")]),
        ("tenet::air_saturates", &[
            ("what", "The air saturates with vapour."),
            ("subject", "air"),
            ("keywords", "air"),
        ]),
        ("tenet::wind_pushes", &[
            ("what", "The wind carries warmth."),
            ("subject", "air"),
            ("keywords", "wind"),
        ]),
        ("tenet::sea_is_deep", &[
            ("what", "Nothing near."),
            ("subject", "air"),
            ("keywords", "sea"),
        ]),
    ])
}

fn run_near(args: &[&str], near: &[Near]) -> mockspace::tool::ToolReport {
    let repo = std::env::temp_dir();
    let crates = std::collections::BTreeSet::new();
    let reg = reg();
    let ctx = mockspace::tool::ToolContext {
        mock_dir: &repo,
        repo_root: &repo,
        all_crates: &crates,
        src_dirs: &[],
        args,
        stdin: None,
        registry: &reg,
    };
    Canon.run_with(&ctx, &|_| near.to_vec())
}

#[test]
fn find_adds_the_tenets_a_near_word_carries_after_the_plain_answer_and_names_the_word() {
    let r = run_near(&["find", "saturation"], &[
        near_of("saturates", false),
        near_of("warmth", true),
    ]);
    let text = answered(&r);
    let plain_end = text.find("near words").expect(text);
    let (plain, tail) = text.split_at(plain_end);
    assert!(plain.contains("0 tenets carry `saturation`"), "{plain}");
    assert!(tail.contains("saturates  (form)"), "{tail}");
    assert!(tail.contains("warmth  (synonym, 1 rows)"), "{tail}");
    assert!(
        tail.contains("air_saturates") && tail.contains("wind_pushes"),
        "{tail}"
    );
    // THE CONTROL: a tenet that carries neither near word is not listed.
    assert!(!tail.contains("sea_is_deep"), "{tail}");
}

#[test]
fn a_tenet_the_plain_answer_already_listed_is_not_listed_again_among_the_near_words() {
    let r = run_near(&["find", "saturates"], &[
        near_of("saturated", false),
        near_of("air", true),
    ]);
    let text = answered(&r);
    let plain_end = text.find("near words").unwrap_or(text.len());
    let tail = &text[plain_end ..];
    assert!(text[.. plain_end].contains("air_saturates"), "{text}");
    assert!(!tail.contains("air_saturates"), "{tail}");
}

#[test]
fn without_a_near_word_find_answers_exactly_as_it_did() {
    let with = run_near(&["find", "wind"], &[]);
    let text = answered(&with);
    assert!(!text.contains("near words"), "{text}");
    // Other verbs never ask for near words.
    let sub = run_near(&["subject", "air"], &[near_of("saturates", false)]);
    assert!(!answered(&sub).contains("near words"));
}
