//! The filters a search was asked for, each one of the index's generic filters, built over the
//! index where it needs to know what the index holds.

use mockspace::tool::ToolContext;
use muisti::repo::filter::{
    Classes,
    Linked,
    Meaning,
    Namespace,
    OfSubject,
    Phrase,
    Since,
    Spoken,
    Under,
    Until,
};
use muisti::repo::{Filter, RepoIndex};

use crate::spec::Spec;

/// Every filter the search names, as one list each answer has to pass.
///
/// # Errors
/// What the index said when a subject's or a row's links could not be read.
pub fn build(
    spec: &Spec,
    index: &RepoIndex,
    ctx: &ToolContext<'_>,
) -> Result<Vec<Box<dyn Filter>>, String> {
    let mut out: Vec<Box<dyn Filter>> = Vec::new();
    if !spec.classes.is_empty() {
        out.push(Box::new(Classes(spec.classes.clone())));
    }
    if let Some(prefix) = &spec.under {
        out.push(Box::new(Under(prefix.clone())));
    }
    if let Some(day) = spec.since {
        out.push(Box::new(Since(day)));
    }
    if let Some(day) = spec.until {
        out.push(Box::new(Until(day)));
    }
    if spec.spoken {
        out.push(Box::new(Spoken));
    }
    if spec.phrase {
        out.push(Box::new(Phrase));
    }
    if spec.meaning {
        out.push(Box::new(Meaning));
    }
    if !spec.namespace.is_empty() {
        out.push(Box::new(Namespace(spec.namespace.clone())));
    }
    if let Some(subject) = &spec.subject {
        let f = OfSubject::of(index, subject)
            .map_err(|e| format!("the rows of subject `{subject}` could not be read: {e}"))?;
        out.push(Box::new(f));
    }
    if let Some(key) = &spec.linked {
        let key = qualified(key, ctx);
        let f = Linked::to(index, &key)
            .map_err(|e| format!("the links of `{key}` could not be read: {e}"))?;
        out.push(Box::new(f));
    }
    Ok(out)
}

/// A row named bare is looked up in every namespace the registry has, and the first that holds it
/// names it; one named with its namespace stays as it is.
fn qualified(key: &str, ctx: &ToolContext<'_>) -> String {
    if key.contains("::") {
        return key.to_string();
    }
    ctx.registry
        .namespaces()
        .map(|ns| format!("{ns}::{key}"))
        .find(|q| ctx.registry.row(q).is_some())
        .unwrap_or_else(|| key.to_string())
}
