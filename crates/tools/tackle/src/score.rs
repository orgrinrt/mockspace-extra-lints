//! What makes a subject the next one, scored rather than filtered.
//!
//! Ranking topics by one number, row count, against one threshold, and switching
//! to a second question only when the first is exhausted, sends work to the wrong
//! place, so every signal is weighed together. A single count cannot tell a topic
//! nobody has started from one that is argued out and untranscribed, and those
//! want opposite work.
//!
//! Five signals, each normalised across the topics so they are comparable, then
//! weighted and summed. Every one is printed per topic, so the ranking is
//! arguable rather than believed.

use std::collections::BTreeMap;

/// One topic's raw measurements, before anything is normalised.
#[derive(Default, Clone)]
pub struct Raw {
    /// Rows anywhere in the registry naming this topic.
    pub rows:       usize,
    /// Rows naming this subject that are superseded, and so not counted in `rows`.
    pub superseded: usize,
    /// Rows carrying a `gap`: a hole somebody wrote down.
    pub gaps:       usize,
    /// Outbound references from this topic's rows.
    pub edges:      usize,
    /// Research mass scoring for this topic's own vocabulary.
    pub corpus:     f64,
    /// Whether it is about the project rather than about the machine.
    pub about_work: bool,
}

/// One topic, scored.
pub struct Scored {
    pub topic:   String,
    pub raw:     Raw,
    /// Each signal after normalisation, in the order [`WEIGHTS`] names them.
    pub signals: [f64; 5],
    pub score:   f64,
}

/// What each signal is worth, and what it is.
///
/// The numbers are judgements and they are here rather than buried so they can
/// be argued with. What is not a judgement is that all five are present: a
/// ranking on any one of them alone sends work to the wrong place, and the two
/// heaviest are the two that disagree most often.
pub const WEIGHTS: [(&str, f64); 5] = [
    // How little is written. The original ranking, kept as one voice of five.
    ("thin", 0.28),
    // How much research exists that the registry has not absorbed. **The signal
    // the first version had no way to see**, and the one that would have caught
    // a data store specified across four documents and carried by one pointer
    // row. A topic scoring high here needs transcription rather than thought.
    ("unread", 0.30),
    // How much of the subject has been superseded. A subject whose rows keep
    // being replaced is the nearest thing to unsettled.
    ("moving", 0.16),
    // Holes somebody wrote down: rows marking something left open. Cheap to act
    // on and easy to forget, because a hole is invisible to anything counting rows.
    ("gaps", 0.16),
    // Rows that point at nothing. A topic whose parts do not reference each
    // other is a pile of answers rather than a design.
    ("islands", 0.10),
];

/// A topic about the project is scored, ranked, printed, and never picked.
///
/// **Not weighed down. Excluded from the pick, and that is a decision that
/// predates this rewrite.** The reason it was made: how the work is organised is
/// the cheapest thing in the room to have an opinion about, so a ranking that
/// lets those topics compete points every unit at them forever.
///
/// A multiplier was tried here instead, on the reasoning that a rule hiding a
/// subject forever is the same shape as a tool reporting an absence it never
/// checked. **It failed its own test on the first realistic fixture**: a process
/// topic with one row against real subjects carrying forty and sixty took the
/// top, because one row is genuinely the thinnest thing in the registry and no
/// multiplier short of zero survives that gap. Which is the original reason
/// restated with a number on it.
///
/// So they stay in the table with their real scores, because what they cost
/// should be visible, and the pick walks past them.
pub const PICKABLE: fn(&Scored) -> bool = |s| !s.raw.about_work;

/// Normalise a column of badness to 0..1 across its own range.
///
/// Across the range rather than by the maximum, and the difference is not
/// cosmetic. Dividing by the maximum sends a column where every topic is equal
/// to **one** everywhere, which adds a constant to every score: harmless for the
/// ordering and wrong for [`dominant`], which then names an axis that decided
/// nothing as the reason a topic was picked. Caught by its own test, where a
/// topic placed by its written-down holes was reported as placed by unread
/// research because every topic happened to carry the same research per row.
///
/// Across the range, a flat column is **zero** everywhere and contributes
/// nothing, which is what an axis nobody varies on is worth. The cost is that
/// every score is relative to the field: the best topic on an axis scores zero
/// there whether it is good in absolute terms or merely least bad. That is the
/// right trade for a tool whose whole question is which subject is most owed
/// work rather than whether any of them is finished.
fn normalise(values: &[f64]) -> Vec<f64> {
    let max = values.iter().copied().fold(f64::MIN, f64::max);
    let min = values.iter().copied().fold(f64::MAX, f64::min);
    let range = max - min;
    if range <= f64::EPSILON {
        return vec![0.0; values.len()];
    }
    values.iter().map(|v| (v - min) / range).collect()
}

/// Score every topic against every other.
///
/// Inverted where a low number is the bad one: a topic with few rows scores high
/// on `thin`, a topic with many edges scores low on `islands`.
#[must_use]
pub fn rank(raws: &BTreeMap<String, Raw>) -> Vec<Scored> {
    let topics: Vec<&String> = raws.keys().collect();
    if topics.is_empty() {
        return Vec::new();
    }

    // Each axis as raw badness first, higher meaning more owed, then all five
    // through the same normaliser. Uniformly, because an axis scaled its own way
    // is an axis nobody can compare against the others, and the weights below
    // only mean anything if the five arrive on one scale.
    #[allow(clippy::cast_precision_loss)]
    let thin: Vec<f64> = topics.iter().map(|t| -(raws[*t].rows as f64)).collect();

    // Research mass per row already written. A topic with a lot of research and
    // few rows is untranscribed; one with a lot of both is being worked.
    // Divided rather than raw, or every large subject scores high forever
    // however well it is recorded.
    let unread: Vec<f64> = topics
        .iter()
        .map(|t| {
            let r = &raws[*t];
            #[allow(clippy::cast_precision_loss)]
            let rows = r.rows as f64;
            r.corpus / (1.0 + rows)
        })
        .collect();

    // The share of everything ever written under the subject that has been
    // superseded. A subject with nothing written has nothing moving.
    let moving: Vec<f64> = topics
        .iter()
        .map(|t| {
            let r = &raws[*t];
            let all = r.rows + r.superseded;
            if all == 0 {
                return 0.0;
            }
            #[allow(clippy::cast_precision_loss)]
            let share = r.superseded as f64 / all as f64;
            share
        })
        .collect();

    #[allow(clippy::cast_precision_loss)]
    let gaps: Vec<f64> = topics.iter().map(|t| raws[*t].gaps as f64).collect();

    // Edges per row, inverted. Per row rather than absolute, so a big topic is
    // not punished for having more of everything.
    let islands: Vec<f64> = topics
        .iter()
        .map(|t| {
            let r = &raws[*t];
            if r.rows == 0 {
                return 1.0;
            }
            #[allow(clippy::cast_precision_loss)]
            let per_row = r.edges as f64 / r.rows as f64;
            1.0 / (1.0 + per_row)
        })
        .collect();

    let thin = normalise(&thin);
    let unread = normalise(&unread);
    let moving = normalise(&moving);
    let gaps = normalise(&gaps);
    let islands = normalise(&islands);

    let mut out: Vec<Scored> = Vec::with_capacity(topics.len());
    for (i, t) in topics.iter().enumerate() {
        let signals = [thin[i], unread[i], moving[i], gaps[i], islands[i]];
        let score: f64 = signals
            .iter()
            .zip(WEIGHTS.iter())
            .map(|(s, (_, w))| s * w)
            .sum();
        out.push(Scored {
            topic: (*t).clone(),
            raw: raws[*t].clone(),
            signals,
            score,
        });
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.topic.cmp(&b.topic))
    });
    out
}

/// Which signal drove a topic's placement, for the brief to say so.
///
/// The instruction a unit is given differs by signal: a thin topic wants
/// research and argument, an unread one wants somebody to go and read what is
/// already there. Naming the wrong one sends a unit to derive from scratch an
/// answer that is sitting in a panel file, which is the exact failure this
/// arm was added for.
#[must_use]
pub fn dominant(s: &Scored) -> &'static str {
    let mut best = 0usize;
    let mut best_v = -1.0f64;
    for (i, (v, (_, w))) in s.signals.iter().zip(WEIGHTS.iter()).enumerate() {
        let contribution = v * w;
        if contribution > best_v {
            best_v = contribution;
            best = i;
        }
    }
    WEIGHTS[best].0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A topic with the given measurements and everything else at zero.
    fn raw(rows: usize, superseded: usize, gaps: usize, edges: usize, corpus: f64) -> Raw {
        Raw {
            rows,
            superseded,
            gaps,
            edges,
            corpus,
            about_work: false,
        }
    }

    fn ranked(pairs: &[(&str, Raw)]) -> Vec<String> {
        let map: BTreeMap<String, Raw> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect();
        rank(&map).into_iter().map(|s| s.topic).collect()
    }

    #[test]
    fn the_signal_the_tool_was_rebuilt_for_can_win_on_its_own() {
        // Two topics identical but for research behind them. The one with a
        // body of work nobody transcribed must come first, and the old
        // row-count ranking could not tell them apart at all.
        let order = ranked(&[
            ("transcribed", raw(10, 5, 1, 10, 5.0)),
            ("unabsorbed", raw(10, 5, 1, 10, 900.0)),
        ]);
        assert_eq!(order.first().map(String::as_str), Some("unabsorbed"));
    }

    #[test]
    fn a_thin_topic_still_wins_against_a_fat_one_when_nothing_else_differs() {
        // The original behaviour, kept as one voice of five rather than
        // discarded. A rewrite that lost it would be a different tool.
        let order = ranked(&[("fat", raw(50, 25, 2, 50, 100.0)), ("thin", raw(2, 1, 2, 2, 100.0))]);
        assert_eq!(order.first().map(String::as_str), Some("thin"));
    }

    #[test]
    fn holes_somebody_wrote_down_move_a_topic_up() {
        let order =
            ranked(&[("clean", raw(20, 10, 0, 20, 50.0)), ("holed", raw(20, 10, 18, 20, 50.0))]);
        assert_eq!(order.first().map(String::as_str), Some("holed"));
    }

    #[test]
    fn a_subject_still_moving_outranks_a_settled_one() {
        // The signal reads how much of a subject has been superseded.
        let order =
            ranked(&[("settled", raw(20, 0, 2, 20, 50.0)), ("moving", raw(20, 20, 2, 20, 50.0))]);
        assert_eq!(order.first().map(String::as_str), Some("moving"));
    }

    #[test]
    fn rows_pointing_nowhere_move_a_topic_up() {
        let order =
            ranked(&[("joined", raw(20, 10, 2, 80, 50.0)), ("islands", raw(20, 10, 2, 0, 50.0))]);
        assert_eq!(order.first().map(String::as_str), Some("islands"));
    }

    // --- the controls ---------------------------------------------------------

    #[test]
    fn a_topic_about_the_work_is_scored_and_ranked_and_never_picked() {
        // All three halves of the contract, because dropping any one of them
        // was a live option and two were tried. It keeps a real score, so what
        // it costs is visible; it takes its real place in the ranking, so the
        // table is not a lie; and the pick walks past it however bad it looks.
        let mut about = raw(1, 0, 9, 0, 900.0);
        about.about_work = true;
        let map: BTreeMap<String, Raw> = [
            ("process".to_string(), about),
            ("boot_and_trust".to_string(), raw(40, 20, 4, 40, 100.0)),
            ("front_end".to_string(), raw(60, 55, 1, 200, 20.0)),
        ]
        .into_iter()
        .collect();
        let out = rank(&map);
        assert_eq!(out[0].topic, "process", "it still ranks where it belongs");
        assert!(out[0].score > 0.0, "and carries a real score, not a zero");
        let picked = out.iter().find(|s| PICKABLE(s)).map(|s| s.topic.as_str());
        assert_eq!(picked, Some("boot_and_trust"), "but the pick walks past it");
    }

    #[test]
    fn topics_that_differ_in_nothing_do_not_get_a_ranking_out_of_noise() {
        // Every axis flat. A normaliser dividing by a zero range, or one that
        // let floating point decide, would order these arbitrarily and a reader
        // would act on it.
        let order = ranked(&[
            ("a", raw(10, 5, 1, 10, 50.0)),
            ("b", raw(10, 5, 1, 10, 50.0)),
            ("c", raw(10, 5, 1, 10, 50.0)),
        ]);
        assert_eq!(
            order,
            vec!["a", "b", "c"],
            "ties break by name, not by chance"
        );
    }

    #[test]
    fn an_empty_registry_ranks_nothing_rather_than_dividing_by_zero() {
        assert!(rank(&BTreeMap::new()).is_empty());
    }

    #[test]
    fn a_corpus_that_was_never_read_leaves_the_other_four_deciding() {
        // Every topic at zero research, which is what happens when the corpus
        // cannot be loaded. The ranking must still work off the registry rather
        // than collapsing, and the tool says separately that the signal is
        // missing.
        let order = ranked(&[("thin", raw(2, 1, 3, 0, 0.0)), ("fat", raw(40, 30, 1, 90, 0.0))]);
        assert_eq!(order.first().map(String::as_str), Some("thin"));
    }

    #[test]
    fn the_named_driver_is_the_one_that_actually_placed_it() {
        // The brief branches on this, so a wrong name sends a research brief at
        // a subject whose problem is that nobody read the research.
        let map: BTreeMap<String, Raw> = [
            ("unabsorbed".to_string(), raw(10, 5, 0, 10, 900.0)),
            ("ordinary".to_string(), raw(10, 5, 0, 10, 1.0)),
        ]
        .into_iter()
        .collect();
        let out = rank(&map);
        assert_eq!(dominant(&out[0]), "unread");

        // Everything flat but the holes. This is the case that caught the
        // normaliser: dividing by the maximum gave the flat research axis a
        // one for both topics, so `dominant` reported a subject placed by its
        // holes as placed by unread research.
        let map: BTreeMap<String, Raw> = [
            ("holed".to_string(), raw(30, 28, 30, 60, 1.0)),
            ("other".to_string(), raw(30, 28, 0, 60, 1.0)),
        ]
        .into_iter()
        .collect();
        let out = rank(&map);
        assert_eq!(dominant(&out[0]), "gaps");
    }

    #[test]
    fn the_weights_sum_to_one_so_a_score_is_readable_as_a_share() {
        let total: f64 = WEIGHTS.iter().map(|(_, w)| w).sum();
        assert!((total - 1.0).abs() < 1e-9, "weights sum to {total}");
    }
}
