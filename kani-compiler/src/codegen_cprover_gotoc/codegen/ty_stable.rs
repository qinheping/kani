// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Stable functions involving type manipulation.
//!
//! This may for now invoke functions that use internal Rust compiler APIs.
//! While we migrate to stable APIs, this module will contain stable versions of functions from
//! `typ.rs`.

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Type;
use rustc_middle::mir;
use rustc_middle::mir::visit::{MutVisitor, NonUseContext, PlaceContext};
use rustc_middle::mir::Place as PlaceInternal;
use rustc_middle::ty::{Ty as TyInternal, TyCtxt};
use rustc_smir::rustc_internal;
use stable_mir::mir::{Local, Place};
use stable_mir::ty::{RigidTy, Ty, TyKind};

impl<'tcx> GotocCtx<'tcx> {
    pub fn place_ty_stable(&self, place: &Place) -> Ty {
        place.ty(self.current_fn().body().locals()).unwrap()
    }

    pub fn codegen_ty_stable(&mut self, ty: Ty) -> Type {
        self.codegen_ty(rustc_internal::internal(ty))
    }

    pub fn local_ty_stable(&mut self, local: Local) -> Ty {
        self.current_fn().body().local_decl(local).unwrap().ty
    }
}
/// If given type is a Ref / Raw ref, return the pointee type.
pub fn pointee_type(mir_type: Ty) -> Option<Ty> {
    match mir_type.kind() {
        TyKind::RigidTy(RigidTy::Ref(_, pointee_type, _)) => Some(pointee_type),
        TyKind::RigidTy(RigidTy::RawPtr(ty, ..)) => Some(ty),
        _ => None,
    }
}

/// Convert internal rustc's structs into StableMIR ones.
///
/// The body of a StableMIR instance already comes monomorphized, which is different from rustc's
/// internal representation. To allow us to migrate parts of the code generation stage with
/// smaller PRs, we have to instantiate rustc's components when converting them to stable.
///
/// Once we finish migrating the entire function code generation, we can remove this code.
pub struct StableConverter<'a, 'tcx> {
    gcx: &'a GotocCtx<'tcx>,
}

impl<'a, 'tcx> StableConverter<'a, 'tcx> {
    pub fn convert_place(gcx: &'a GotocCtx<'tcx>, mut place: PlaceInternal<'tcx>) -> Place {
        let mut converter = StableConverter { gcx };
        converter.visit_place(
            &mut place,
            PlaceContext::NonUse(NonUseContext::VarDebugInfo),
            mir::Location::START,
        );
        rustc_internal::stable(place)
    }
}

impl<'a, 'tcx> MutVisitor<'tcx> for StableConverter<'a, 'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.gcx.tcx
    }

    fn visit_ty(&mut self, ty: &mut TyInternal<'tcx>, _: mir::visit::TyContext) {
        *ty = self.gcx.monomorphize(*ty);
    }
}