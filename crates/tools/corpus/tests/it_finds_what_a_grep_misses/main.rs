//! What this instrument must find, and what it must refuse to find.
//!
//! Every test plants a corpus whose answer is known before the search runs, and
//! the negative controls are the point rather than the decoration: a search that
//! cannot come back empty is not a search, and this one was built to correct a
//! tool that never did.

use std::path::{Path, PathBuf};

use corpus::{Corpus, Index, Query, Source, rank, terms};

/// A corpus on disk, removed when the test ends.
struct Planted(PathBuf);

impl Planted {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kaski-corpus-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("research")).unwrap();
        Self(dir)
    }

    fn file(&self, rel: &str, body: &str) -> &Self {
        let p = self.0.join("research").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
        self
    }

    fn mock(&self) -> &Path {
        &self.0
    }
}

impl Drop for Planted {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn search(dir: &Planted, phrase: &str, keywords: &[&str]) -> Vec<String> {
    let corpus = Corpus::load(dir.mock());
    let mut q = Query::new(phrase);
    for k in keywords {
        q.push((*k).to_string(), Source::Keyword);
    }
    rank(&corpus, &Index::build(&corpus), &q, 20)
        .hits
        .into_iter()
        .map(|h| h.path)
        .collect()
}

mod counting;
mod finding;
mod kaski;
mod ranking;
mod reporting;
mod stemming;
mod tokenising;

/// The whole search, for the tests that are about what a caller may conclude
/// rather than about which files came back.
fn run(dir: &Planted, phrase: &str) -> corpus::Search {
    let corpus = Corpus::load(dir.mock());
    rank(&corpus, &Index::build(&corpus), &Query::new(phrase), 20)
}
