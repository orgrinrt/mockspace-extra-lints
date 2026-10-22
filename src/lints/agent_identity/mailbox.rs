//! A mailbox as the parts a given-name tool is judged on.

use super::lists::{MACHINE_SUFFIXES, VENDOR_DOMAINS};

/// The local part and the domain of a mailbox. A value with no `@` has neither.
#[derive(Default)]
pub(super) struct Mailbox {
    local:  String,
    domain: String,
}

impl Mailbox {
    /// Split `mailbox` at its last `@`.
    pub(super) fn of(mailbox: &str) -> Self {
        let Some((local, domain)) = mailbox.rsplit_once('@') else {
            return Self::default();
        };
        Self {
            local:  local.to_string(),
            domain: domain.to_string(),
        }
    }

    /// Whether the domain is a machine's: `localhost` or any domain of one label,
    /// or one ending in a machine suffix, a dot before it. A person is called the
    /// same at an ordinary domain, so a local part is a signal only at a
    /// machine's. GitHub's private address, which every account has, is an
    /// ordinary domain.
    fn is_a_machines(&self) -> bool {
        !self.domain.is_empty()
            && (!self.domain.contains('.')
                || MACHINE_SUFFIXES
                    .iter()
                    .any(|suffix| self.domain.ends_with(&format!(".{suffix}"))))
    }

    /// Whether the mailbox is a given-name tool's: its domain is one of the tool's
    /// vendor domains, or its local part is the tool's word at a machine's
    /// mailbox. One with no domain is nobody's.
    pub(super) fn is_the_tools(&self, tool: &str) -> bool {
        !self.domain.is_empty()
            && ((self.local == tool && self.is_a_machines())
                || VENDOR_DOMAINS
                    .iter()
                    .any(|(t, d)| *t == tool && *d == self.domain))
    }

    /// Whether the domain is one a given-name tool commits from, whichever tool.
    pub(super) fn is_at_a_vendor(&self) -> bool {
        !self.domain.is_empty() && VENDOR_DOMAINS.iter().any(|(_, d)| *d == self.domain)
    }
}
