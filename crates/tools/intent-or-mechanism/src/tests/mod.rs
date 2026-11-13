//! Two subjects, and neither reaches the other.
//!
//! `schema` is about reading `mockspace.toml`: which fields count as a
//! statement, which namespace a field block belongs to, and how a name is
//! matched. `run` goes through the tool, because a person invokes that and
//! nothing below it tests which rows are read or what the argument narrows.

mod kaski;
mod run;
mod schema;
