//! An opt-in mockspace lint pack: shared lints and presets a project imports
//! because it wants them, rather than anything mockspace ships by default.
//!
//! The stack lints (no-alloc, no-std, the bare-primitive family, the arvo and
//! strategy-marker rules) came first and are why this existed, but nothing here
//! is limited to that stack, and the commit-style and forge-body presets it also
//! carries are general.
//!
//! Consumed by a repo's `mockspace.toml` via:
//!
//! ```toml
//! [lint-crates]
//! mockspace-extra-lints = { path = "../mockspace-extra-lints" }
//! ```
//!
//! Emitting every lint from one place is the point: policy stays in sync across
//! importers instead of drifting per repo, which is exactly what happened to the
//! hand-copied commit-style rule before it moved here.

mod util;

pub mod lints {
    //! Individual lint rules. Each is a unit struct that implements
    //! `mockspace_lint_rules::CrateLint` or `WorkspaceLint`, over the shared
    //! `Lint` supertrait.

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
    pub mod strategy_marker_required;
    pub mod semantic_alias_nudge;
    pub mod trait_first_signatures;
    pub mod arvo_types_only;
    pub mod lint_allow_requires_task_id;
    pub mod writing_style;
}

use lints::{
    arvo_types_only::ArvoTypesOnly, lint_allow_requires_task_id::LintAllowRequiresTaskId,
    no_alloc::NoAlloc, no_bare_numeric::NoBareNumeric, no_bare_option::NoBareOption,
    no_bare_result::NoBareResult, no_bare_static_str::NoBareStaticStr,
    no_bare_string::NoBareString, no_dyn_dispatch::NoDynDispatch,
    no_public_raw_field::NoPublicRawField, no_runtime_registration::NoRuntimeRegistration,
    no_runtime_spawn::NoRuntimeSpawn, no_std::NoStd, no_vec_in_trait_sig::NoVecInTraitSig,
    semantic_alias_nudge::SemanticAliasNudge, strategy_marker_required::StrategyMarkerRequired,
    trait_first_signatures::TraitFirstSignatures, writing_style::WritingStyle,
};

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
        StrategyMarkerRequired,
        SemanticAliasNudge,
        TraitFirstSignatures,
        ArvoTypesOnly,
        LintAllowRequiresTaskId,
    ],
    workspace_lints: [
        WritingStyle,
    ],
}
