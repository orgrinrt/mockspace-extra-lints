//! An opt-in mockspace lint pack, shared lints and presets a project imports
//! because it wants them rather than anything mockspace ships by default.
//!
//! Some of it is narrow, the bare-primitive family and the arvo and
//! strategy-marker rules only mean something to a codebase built on that layer,
//! and some of it is general, the commit-style and forge-body ones fit any
//! project that has an opinion about its own git text.
//!
//! Consumed by a repo's `mockspace.toml`:
//!
//! ```toml
//! [lint-crates]
//! mockspace-extra-lints = { git = "ssh://git@github.com/orgrinrt/mockspace-extra-lints.git", branch = "dev" }
//! ```
//!
//! A path dependency works the same way where the two sit beside each other in
//! one workspace.
//!
//! Emitting every lint from one place is the point, since a policy several repos
//! share stays in step that way, where a copy per repo drifts and nothing says
//! which of the copies is the one that is right.

mod const_generic_parameters;
mod dep_surface;
mod paths;
pub mod tools;
mod util;

pub mod lints {
    //! Individual lint rules. Each is a unit struct over the shared
    //! `mockspace_lint_rules::Lint` supertrait, and then one of `CrateLint`,
    //! `WorkspaceLint` or `MessageLint` depending on what it is handed: a crate
    //! with its sources, the workspace as a whole, or one commit message or
    //! forge body.

    pub mod commit_style;
    pub mod forge_body;
    pub mod message_attribution;
    pub mod no_alloc;
    pub mod no_std;
    pub mod no_bare_option;
    pub mod no_bare_result;
    pub mod no_bare_numeric;
    pub mod no_bare_string;
    pub mod no_bare_static_str;
    pub mod no_dyn_dispatch;
    pub mod no_runtime_spawn;
    pub mod no_runtime_registration;
    pub mod no_public_raw_field;
    pub mod no_vec_in_trait_sig;
    pub mod re_export_foreign_names;
    pub mod strategy_marker_required;
    pub mod semantic_alias_nudge;
    pub mod trait_first_signatures;
    pub mod arvo_types_only;
    pub mod lint_allow_requires_task_id;
    pub mod writing_style;
}

use lints::{
    arvo_types_only::ArvoTypesOnly,
    commit_style::CommitStyle,
    forge_body::ForgeBody,
    message_attribution::MessageAttribution, lint_allow_requires_task_id::LintAllowRequiresTaskId,
    no_alloc::NoAlloc, no_bare_numeric::NoBareNumeric, no_bare_option::NoBareOption,
    no_bare_result::NoBareResult, no_bare_static_str::NoBareStaticStr,
    no_bare_string::NoBareString, no_dyn_dispatch::NoDynDispatch,
    no_public_raw_field::NoPublicRawField, no_runtime_registration::NoRuntimeRegistration,
    no_runtime_spawn::NoRuntimeSpawn, no_std::NoStd, no_vec_in_trait_sig::NoVecInTraitSig,
    re_export_foreign_names::ReExportForeignNames, semantic_alias_nudge::SemanticAliasNudge,
    strategy_marker_required::StrategyMarkerRequired,
    trait_first_signatures::TraitFirstSignatures, writing_style::WritingStyle,
};
use crate::tools::coverage::Coverage;
use crate::tools::rulings_with_no_verbatim::RulingsWithNoVerbatim;

mockspace_lint_rules::lint_pack! {
    lints: [
        NoAlloc,
        NoStd,
        NoBareOption,
        NoBareResult,
        NoBareNumeric,
        NoBareString,
        NoBareStaticStr,
        NoDynDispatch,
        NoRuntimeSpawn,
        NoRuntimeRegistration,
        NoPublicRawField,
        NoVecInTraitSig,
        ReExportForeignNames,
        StrategyMarkerRequired,
        SemanticAliasNudge,
        TraitFirstSignatures,
        ArvoTypesOnly,
        LintAllowRequiresTaskId,
    ],
    workspace_lints: [
        WritingStyle,
    ],
    // Both carry configuration, so they are constructed rather than named as
    // unit structs. The macro takes expressions for exactly this case.
    message_lints: [
        CommitStyle::default(),
        ForgeBody::default(),
        MessageAttribution::default(),
    ],
    // Tools ride the same cdylib as the lints and `mock tools` enumerates them
    // identically, so a consumer gets these by depending on the pack and needs
    // no `<mock>/tools/` directory of its own. See `tools` for which checks
    // belong here rather than in one repository.
    tools: [
        Coverage,
        RulingsWithNoVerbatim,
    ],
}
