//! The verdict store the replay tests of the lints that save per file share.

use std::cell::RefCell;
use std::collections::HashMap;

use mockspace_lint_rules::verdict::VerdictCache;

/// A store that keeps what it is given and counts how each ask was answered, so a test can say that a
/// warm run replayed and not only that it found the same things.
#[derive(Default)]
pub(crate) struct Kept {
    saved: RefCell<HashMap<String, String>>,
    asked: RefCell<HashMap<(String, bool), usize>>,
}

impl Kept {
    /// How many asks under `lint` were answered from the store since the counts were last cleared.
    pub(crate) fn hits(&self, lint: &str) -> usize {
        self.asked(lint, true)
    }

    /// How many asks under `lint` were judged again since the counts were last cleared.
    pub(crate) fn misses(&self, lint: &str) -> usize {
        self.asked(lint, false)
    }

    /// Forget the counts and keep what is saved, so the next run is counted alone.
    pub(crate) fn clear_counts(&self) {
        self.asked.borrow_mut().clear();
    }

    fn asked(&self, lint: &str, hit: bool) -> usize {
        self.asked.borrow().get(&(lint.to_string(), hit)).copied().unwrap_or(0)
    }
}

impl VerdictCache for Kept {
    fn digest(&self, parts: &[&[u8]]) -> String {
        parts.iter().map(|p| format!("{}:{}|", p.len(), String::from_utf8_lossy(p))).collect()
    }

    fn replay(&self, key: &str) -> Option<String> {
        self.saved.borrow().get(key).cloned()
    }

    fn remember(&self, key: &str, encoded: &str) {
        self.saved.borrow_mut().insert(key.to_string(), encoded.to_string());
    }

    fn tally(&self, hit: bool, lint: &str) {
        *self.asked.borrow_mut().entry((lint.to_string(), hit)).or_default() += 1;
    }
}
