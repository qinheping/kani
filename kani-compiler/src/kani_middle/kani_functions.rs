// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for handling special functions from the Kani library.
//!
//! There are three types of functions today:
//!    1. Kani intrinsics: These are functions whose MIR body is generated during compilation time.
//!       Their body usually requires some extra knowledge about the given types
//!       that's only available during compilation.
//!    2. Kani models: These are functions that model a specific behavior that is of interest of
//!       the compiler. The body of the model is kept as is, and the compiler may invoke the model
//!       to perform some operation, such as, retrieving information about memory initialization.
//!    3. Kani hooks: These are similar to Kani intrinsics in a sense that their body will be
//!       generated by the compiler. However, their body exposes some backend specific logic, thus,
//!       their bodies are generated as part of the codegen stage.
//!       From a Kani library perspective, there is no difference between hooks and intrinsics.
//!       This is a compiler implementation detail, but for simplicity we use the "Hook" suffix
//!       in their names.
//!
//! These special functions are marked with `kanitool::fn_marker` attribute attached to them.
//! The marker value will contain "Intrinsic", "Model", or "Hook" suffix, indicating which category
//! they fit in.

use crate::kani_middle::attributes;
use stable_mir::mir::mono::Instance;
use stable_mir::ty::FnDef;
use std::collections::HashMap;
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString, IntoStaticStr};
use tracing::debug;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum KaniFunction {
    Model(KaniModel),
    Intrinsic(KaniIntrinsic),
    Hook(KaniHook),
}

/// Kani intrinsics are functions generated by the compiler.
///
/// These functions usually depend on information that require extra knowledge about the type
/// or extra Kani instrumentation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoStaticStr, EnumIter, EnumString, Hash)]
pub enum KaniIntrinsic {
    #[strum(serialize = "AnyModifiesIntrinsic")]
    AnyModifies,
    #[strum(serialize = "CheckedAlignOfIntrinsic")]
    CheckedAlignOf,
    #[strum(serialize = "CheckedSizeOfIntrinsic")]
    CheckedSizeOf,
    #[strum(serialize = "IsInitializedIntrinsic")]
    IsInitialized,
    #[strum(serialize = "ValidValueIntrinsic")]
    ValidValue,
    #[strum(serialize = "WriteAnyIntrinsic")]
    WriteAny,
}

/// Kani models are Rust functions that model some runtime behavior used by Kani instrumentation.
#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoStaticStr, EnumIter, EnumString, Hash)]
pub enum KaniModel {
    #[strum(serialize = "AlignOfDynObjectModel")]
    AlignOfDynObject,
    #[strum(serialize = "AlignOfValRawModel")]
    AlignOfVal,
    #[strum(serialize = "AnyModel")]
    Any,
    #[strum(serialize = "CopyInitStateModel")]
    CopyInitState,
    #[strum(serialize = "CopyInitStateSingleModel")]
    CopyInitStateSingle,
    #[strum(serialize = "LoadArgumentModel")]
    LoadArgument,
    #[strum(serialize = "InitializeMemoryInitializationStateModel")]
    InitializeMemoryInitializationState,
    #[strum(serialize = "IsPtrInitializedModel")]
    IsPtrInitialized,
    #[strum(serialize = "IsStrPtrInitializedModel")]
    IsStrPtrInitialized,
    #[strum(serialize = "IsSliceChunkPtrInitializedModel")]
    IsSliceChunkPtrInitialized,
    #[strum(serialize = "IsSlicePtrInitializedModel")]
    IsSlicePtrInitialized,
    #[strum(serialize = "OffsetModel")]
    Offset,
    #[strum(serialize = "RunContractModel")]
    RunContract,
    #[strum(serialize = "RunLoopContractModel")]
    RunLoopContract,
    #[strum(serialize = "SetPtrInitializedModel")]
    SetPtrInitialized,
    #[strum(serialize = "SetSliceChunkPtrInitializedModel")]
    SetSliceChunkPtrInitialized,
    #[strum(serialize = "SetSlicePtrInitializedModel")]
    SetSlicePtrInitialized,
    #[strum(serialize = "SetStrPtrInitializedModel")]
    SetStrPtrInitialized,
    #[strum(serialize = "SizeOfDynObjectModel")]
    SizeOfDynObject,
    #[strum(serialize = "SizeOfSliceObjectModel")]
    SizeOfSliceObject,
    #[strum(serialize = "SizeOfValRawModel")]
    SizeOfVal,
    #[strum(serialize = "StoreArgumentModel")]
    StoreArgument,
    #[strum(serialize = "WriteAnySliceModel")]
    WriteAnySlice,
    #[strum(serialize = "WriteAnySlimModel")]
    WriteAnySlim,
    #[strum(serialize = "WriteAnyStrModel")]
    WriteAnyStr,
}

/// Kani hooks are Rust functions defined in the Kani library that exposes some semantics
/// from our solvers / symbolic execution engines.
///
/// Thus, they get handled by the codegen stage.
#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoStaticStr, EnumIter, EnumString, Hash)]
pub enum KaniHook {
    #[strum(serialize = "AnyRawHook")]
    AnyRaw,
    #[strum(serialize = "AssertHook")]
    Assert,
    #[strum(serialize = "AssumeHook")]
    Assume,
    #[strum(serialize = "CheckHook")]
    Check,
    #[strum(serialize = "CoverHook")]
    Cover,
    // TODO: this is temporarily implemented as a hook, but should be implemented as an intrinsic
    #[strum(serialize = "FloatToIntInRangeHook")]
    FloatToIntInRange,
    #[strum(serialize = "InitContractsHook")]
    InitContracts,
    #[strum(serialize = "IsAllocatedHook")]
    IsAllocated,
    #[strum(serialize = "PanicHook")]
    Panic,
    #[strum(serialize = "PointerObjectHook")]
    PointerObject,
    #[strum(serialize = "PointerOffsetHook")]
    PointerOffset,
    #[strum(serialize = "SafetyCheckHook")]
    SafetyCheck,
    #[strum(serialize = "UnsupportedCheckHook")]
    UnsupportedCheck,
    #[strum(serialize = "UntrackedDerefHook")]
    UntrackedDeref,
}

impl From<KaniIntrinsic> for KaniFunction {
    fn from(value: KaniIntrinsic) -> Self {
        KaniFunction::Intrinsic(value)
    }
}

impl From<KaniModel> for KaniFunction {
    fn from(value: KaniModel) -> Self {
        KaniFunction::Model(value)
    }
}

impl From<KaniHook> for KaniFunction {
    fn from(value: KaniHook) -> Self {
        KaniFunction::Hook(value)
    }
}

impl TryFrom<FnDef> for KaniFunction {
    type Error = ();

    fn try_from(def: FnDef) -> Result<Self, Self::Error> {
        let value = attributes::fn_marker(def).ok_or(())?;
        if let Ok(intrinsic) = KaniIntrinsic::from_str(&value) {
            Ok(intrinsic.into())
        } else if let Ok(model) = KaniModel::from_str(&value) {
            Ok(model.into())
        } else if let Ok(hook) = KaniHook::from_str(&value) {
            Ok(hook.into())
        } else {
            Err(())
        }
    }
}

impl TryFrom<Instance> for KaniFunction {
    type Error = ();

    fn try_from(instance: Instance) -> Result<Self, Self::Error> {
        let value = attributes::fn_marker(instance.def).ok_or(())?;
        if let Ok(intrinsic) = KaniIntrinsic::from_str(&value) {
            Ok(intrinsic.into())
        } else if let Ok(model) = KaniModel::from_str(&value) {
            Ok(model.into())
        } else if let Ok(hook) = KaniHook::from_str(&value) {
            Ok(hook.into())
        } else {
            Err(())
        }
    }
}

/// Find all Kani functions.
///
/// First try to find `kani` crate. If that exists, look for the items there.
/// If there's no Kani crate, look for the items in `core` since we could be using `kani_core`.
/// Note that users could have other `kani` crates, so we look in all the ones we find.
pub fn find_kani_functions() -> HashMap<KaniFunction, FnDef> {
    let mut kani = stable_mir::find_crates("kani");
    if kani.is_empty() {
        // In case we are using `kani_core`.
        kani.extend(stable_mir::find_crates("core"));
    }
    debug!(?kani, "find_kani_functions");
    let fns = kani
        .into_iter()
        .find_map(|krate| {
            let kani_funcs: HashMap<_, _> = krate
                .fn_defs()
                .into_iter()
                .filter_map(|fn_def| {
                    KaniFunction::try_from(fn_def).ok().map(|kani_function| {
                        debug!(?kani_function, ?fn_def, "Found kani function");
                        (kani_function, fn_def)
                    })
                })
                .collect();
            // All definitions should live in the same crate, so we can return the first one.
            // If there are no definitions, return `None` to indicate that.
            (!kani_funcs.is_empty()).then_some(kani_funcs)
        })
        .unwrap_or_default();
    if cfg!(debug_assertions) {
        validate_kani_functions(&fns);
    }
    fns
}

/// Ensure we have the valid definitions.
///
/// This is a utility function used to ensure that we have found all the expected functions, as well
/// as to ensure that the cached IDs are up-to-date.
///
/// The second condition is to ensure the cached StableMIR `FnDef` objects are still valid, i.e.:,
/// that we are not storing them past the StableMIR `run` context.
pub fn validate_kani_functions(kani_funcs: &HashMap<KaniFunction, FnDef>) {
    let mut missing = 0u8;
    for func in KaniIntrinsic::iter()
        .map(|i| i.into())
        .chain(KaniModel::iter().map(|m| m.into()))
        .chain(KaniHook::iter().map(|h| h.into()))
    {
        if let Some(fn_def) = kani_funcs.get(&func) {
            assert_eq!(KaniFunction::try_from(*fn_def), Ok(func), "Unexpected function marker");
        } else {
            tracing::error!(?func, "Missing kani function");
            missing += 1;
        }
    }
    assert_eq!(missing, 0, "Failed to find `{missing}` Kani functions");
}
