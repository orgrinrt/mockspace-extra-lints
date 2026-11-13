//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The command lines of the three commands, read into the requests the rules are asked about.

use std::collections::BTreeMap;

use crate::time::{format_utc, now_secs};
use crate::view::Filter;
use crate::write::{AddRequest, MemberWrite, MoveRequest, TagEdit};

/// Flags that take the word after them, for `add`.
pub const ADD_FLAGS: &[&str] = &[
    "--id",
    "--kind",
    "--state",
    "--holder",
    "--by",
    "--at",
    "--why",
    "--what",
    "--options",
    "--needs",
    "--part-of",
    "--superseded-by",
    "--related",
    "--contracts",
    "--shows",
    "--repos",
    "--branch",
    "--brief",
    "--session",
    "--preview",
    "--tags",
    "--rank",
    "--set",
    "--priority",
    "--choice",
];

/// Flags that take the word after them, for `move`.
pub const MOVE_FLAGS: &[&str] = &[
    "--holder",
    "--kind",
    "--related",
    "--contracts",
    "--shows",
    "--by",
    "--at",
    "--why",
    "--answer",
    "--picks",
    "--needs",
    "--part-of",
    "--superseded-by",
    "--tags",
    "--add-tags",
    "--remove-tags",
    "--branch",
    "--brief",
    "--session",
    "--set",
    "--priority",
    "--choice",
    "--span",
    "--core",
    "--for",
];

/// Flags that take the word after them, for the listing.
pub const LIST_FLAGS: &[&str] = &["--state", "--holder", "--kind", "--epoch", "--tag"];

/// The positional words and the flags with their values.
struct Parsed<'a> {
    words: Vec<&'a str>,
    flags: BTreeMap<&'a str, &'a str>,
    bare:  Vec<&'a str>,
}

fn parse<'a>(args: &[&'a str], valued: &[&str]) -> Result<Parsed<'a>, String> {
    let mut p = Parsed {
        words: Vec::new(),
        flags: BTreeMap::new(),
        bare:  Vec::new(),
    };
    let mut it = args.iter().copied();
    while let Some(a) = it.next() {
        if valued.contains(&a) {
            let v = it.next().ok_or_else(|| format!("`{a}` wants a value"))?;
            p.flags.insert(a, v);
        } else if a.starts_with("--") {
            p.bare.push(a);
        } else {
            p.words.push(a);
        }
    }
    Ok(p)
}

fn list(v: Option<&&str>) -> Vec<String> {
    v.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(String::from)
            .collect()
    })
    .unwrap_or_default()
}

/// The options of a question, split on a vertical bar: an option is a label and what it means, and
/// its words hold commas.
fn split_options(v: &str) -> Vec<String> {
    v.split('|')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(String::from)
        .collect()
}

/// The author: `--by`, else the channel identity in the environment.
pub fn by(flag: Option<&&str>) -> String {
    flag.map(|s| (*s).to_string())
        .or_else(|| std::env::var("MSG_AS").ok())
        .unwrap_or_default()
}

pub fn at(flag: Option<&&str>) -> String {
    flag.map_or_else(|| format_utc(now_secs()), |s| (*s).to_string())
}

/// `ticket-add <title> [flags]`.
///
/// # Errors
///
/// When the title is missing, a flag has no value or a flag is not one the command takes.
pub fn add(args: &[&str]) -> Result<AddRequest, String> {
    let p = parse(args, ADD_FLAGS)?;
    if let Some(b) = p.bare.first() {
        return Err(format!("`{b}` is no flag of ticket-add"));
    }
    let title = p.words.join(" ");
    if title.trim().is_empty() {
        return Err("ticket-add wants a title".to_string());
    }
    let f = &p.flags;
    let mut fields = BTreeMap::new();
    for (flag, field, many) in [
        ("--what", "what", false),
        ("--needs", "needs", true),
        ("--part-of", "part_of", false),
        ("--superseded-by", "superseded_by", true),
        ("--related", "related", true),
        ("--contracts", "contracts", true),
        ("--shows", "shows", false),
        ("--repos", "repos", true),
        ("--branch", "branch", false),
        ("--brief", "brief", false),
        ("--session", "session", false),
        ("--preview", "preview", true),
        ("--tags", "tags", true),
        ("--rank", "rank", false),
        ("--set", "set", false),
        ("--priority", "priority", false),
        ("--choice", "choice", false),
    ] {
        if let Some(v) = f.get(flag) {
            let values = if many { list(Some(v)) } else { vec![(*v).to_string()] };
            fields.insert(field.to_string(), values);
        }
    }
    if let Some(v) = f.get("--options") {
        fields.insert("options".to_string(), split_options(v));
    }
    Ok(AddRequest {
        title,
        id: f.get("--id").map(|s| (*s).to_string()),
        kind: f
            .get("--kind")
            .map_or_else(|| "task".to_string(), |s| (*s).to_string()),
        state: f.get("--state").map(|s| (*s).to_string()),
        holder: f
            .get("--holder")
            .map(|s| (*s).to_string())
            .unwrap_or_default(),
        by: by(f.get("--by")),
        at: at(f.get("--at")),
        why: f.get("--why").map(|s| (*s).to_string()).unwrap_or_default(),
        fields,
    })
}

/// `ticket-move <ticket> [<state>] [flags]`: a move, the links of the row, or both.
///
/// # Errors
///
/// When the ticket is missing, a flag has no value, a flag is not one the command takes, or the
/// command is asked for neither a state nor a link.
pub fn move_(args: &[&str]) -> Result<MoveRequest, String> {
    let p = parse(args, MOVE_FLAGS)?;
    if let Some(b) = p.bare.iter().find(|b| **b != "--release") {
        return Err(format!("`{b}` is no flag of ticket-move"));
    }
    let (ticket, to) = match p.words[..] {
        [ticket] => (ticket, ""),
        [ticket, to] => (ticket, to),
        _ => {
            return Err(
                "ticket-move wants a ticket, and the state to move it to unless it only records a link"
                    .to_string(),
            );
        },
    };
    let mut links = BTreeMap::new();
    for (flag, field, many) in [
        ("--kind", "kind", false),
        ("--related", "related", true),
        ("--contracts", "contracts", true),
        ("--shows", "shows", false),
        ("--needs", "needs", true),
        ("--part-of", "part_of", false),
        ("--superseded-by", "superseded_by", true),
        ("--tags", "tags", true),
        ("--branch", "branch", false),
        ("--brief", "brief", false),
        ("--session", "session", false),
        ("--set", "set", false),
        ("--priority", "priority", false),
        ("--choice", "choice", false),
    ] {
        if let Some(v) = p.flags.get(flag) {
            let values = if many { list(Some(v)) } else { vec![(*v).to_string()] };
            links.insert(
                field.to_string(),
                values.into_iter().filter(|x| !x.is_empty()).collect(),
            );
        }
    }
    let span = p.flags.get("--span").map(|s| (*s).to_string());
    let core = match p.flags.get("--core") {
        None => None,
        Some(&"true") => Some(true),
        Some(&"false") => Some(false),
        Some(other) => return Err(format!("`--core` is true or false, not `{other}`")),
    };
    let member = (span.is_some() || core.is_some()).then_some(MemberWrite {
        span,
        core,
    });
    let tags = TagEdit {
        add:    list(p.flags.get("--add-tags")),
        remove: list(p.flags.get("--remove-tags")),
    };
    if to.is_empty() && links.is_empty() && tags.is_empty() && member.is_none() {
        return Err(
            "ticket-move wants a state to move to, or a link or a member to record".to_string(),
        );
    }
    let release = p.bare.contains(&"--release");
    if release && p.flags.contains_key("--holder") {
        return Err("`--release` and `--holder` say opposite things".to_string());
    }
    Ok(MoveRequest {
        ticket: ticket.to_string(),
        to: to.to_string(),
        links,
        member,
        successor: p.flags.get("--for").map(|s| (*s).to_string()),
        tags,
        holder: p.flags.get("--holder").map(|s| (*s).to_string()),
        release,
        by: by(p.flags.get("--by")),
        at: at(p.flags.get("--at")),
        why: p
            .flags
            .get("--why")
            .map(|s| (*s).to_string())
            .unwrap_or_default(),
        answer: p
            .flags
            .get("--answer")
            .map(|s| (*s).to_string())
            .unwrap_or_default(),
        // split on a vertical bar as the options are, since an option holds commas
        picks: p
            .flags
            .get("--picks")
            .map(|v| split_options(v))
            .unwrap_or_default(),
    })
}

/// The verb `tickets` was given, with the word it takes where it takes one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Verb {
    #[default]
    List,
    Answerable,
    Next,
    Check,
    /// The ticket holding this branch.
    Branch(String),
    /// The sha this ticket's branch landed at.
    Landed(String),
    /// The open tickets that change these contract paths.
    Contract(Vec<String>),
    /// Every track with its steps and waits.
    Tracks,
}

/// What `tickets` was asked: the verb and what to list.
#[derive(Debug, Default)]
pub struct ListRequest {
    pub verb:   Verb,
    pub filter: Filter,
    /// Where the tickets are read from: the trunk after a fetch unless `--tree` or `--no-fetch` say otherwise.
    pub opts:   crate::trunk::Opts,
}

/// `tickets <list|next|check> [--state s] [--holder h] [--kind k]`, or `tickets <branch|landed> <word>`;
/// a flag takes comma separated values.
///
/// # Errors
///
/// When no verb is given, the verb is not one of the five, a verb has too few or too many words, a flag
/// has no value or a flag is unknown.
pub fn tickets(args: &[&str]) -> Result<ListRequest, String> {
    let p = parse(args, LIST_FLAGS)?;
    if let Some(b) = p
        .bare
        .iter()
        .find(|b| !matches!(**b, "--tree" | "--no-fetch"))
    {
        return Err(format!("`{b}` is no flag of tickets"));
    }
    let (verb, rest) = match p.words[..] {
        [] => {
            return Err(
                "tickets wants a verb: list, next, check, branch, landed, contract or tracks"
                    .to_string(),
            );
        },
        [v, ref rest @ ..] => (v, rest),
    };
    let one = |what: &str| {
        match rest {
            [w] => Ok((*w).to_string()),
            _ => Err(format!("`{verb}` wants one {what}")),
        }
    };
    let verb = match verb {
        "list" | "next" | "check" | "tracks" | "answerable" if !rest.is_empty() => {
            return Err("tickets takes one verb".to_string());
        },
        "list" => Verb::List,
        "next" => Verb::Next,
        "tracks" => Verb::Tracks,
        "answerable" => Verb::Answerable,
        "check" => Verb::Check,
        "branch" => Verb::Branch(one("branch name")?),
        "landed" => Verb::Landed(one("ticket id")?),
        "contract" if rest.is_empty() => return Err("`contract` wants a contract path".to_string()),
        "contract" => Verb::Contract(rest.iter().map(|w| (*w).to_string()).collect()),
        other => {
            return Err(format!(
                "`{other}` is no verb of tickets; the verbs are `list`, `next`, `check`, `branch`, `landed`, `contract`, `tracks` and `answerable`"
            ));
        },
    };
    Ok(ListRequest {
        verb,
        filter: Filter {
            state:  list(p.flags.get("--state")),
            holder: list(p.flags.get("--holder")),
            kind:   list(p.flags.get("--kind")),
            epoch:  list(p.flags.get("--epoch")),
            tag:    list(p.flags.get("--tag")),
        },
        opts: crate::trunk::Opts {
            tree:  p.bare.contains(&"--tree"),
            fetch: !p.bare.contains(&"--no-fetch"),
        },
    })
}
