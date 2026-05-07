// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Trail-scoped handle wrappers.

mod access;
mod locking;
mod records;
mod tags;

pub(crate) use access::WasmTrailAccess;
use anyhow::anyhow;
use audit_trail::{AuditTrailClient, AuditTrailClientReadOnly};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result, WasmResult};
pub(crate) use locking::WasmTrailLocking;
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::into_transaction_builder;
pub(crate) use records::WasmTrailRecords;
pub(crate) use tags::WasmTrailTags;
use wasm_bindgen::prelude::*;

use crate::trail::{WasmDeleteAuditTrail, WasmMigrate, WasmOnChainAuditTrail, WasmUpdateMetadata};

/// Handle bound to a specific audit-trail object.
///
/// @remarks
/// `AuditTrailHandle` keeps one trail ID together with the originating client so all trail-scoped
/// reads and transaction builders can be discovered from a single value. Use the subsystem
/// accessors {@link AuditTrailHandle.records}, {@link AuditTrailHandle.access},
/// {@link AuditTrailHandle.locking}, and {@link AuditTrailHandle.tags} to reach the corresponding
/// APIs.
#[derive(Clone)]
#[wasm_bindgen(js_name = AuditTrailHandle, inspectable)]
pub struct WasmAuditTrailHandle {
    pub(crate) read_only: AuditTrailClientReadOnly,
    pub(crate) full: Option<AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
}

impl WasmAuditTrailHandle {
    pub(crate) fn from_read_only(read_only: AuditTrailClientReadOnly, trail_id: ObjectID) -> Self {
        Self {
            read_only,
            full: None,
            trail_id,
        }
    }

    pub(crate) fn from_full(full: AuditTrailClient<WasmTransactionSigner>, trail_id: ObjectID) -> Self {
        Self {
            read_only: full.read_only().clone(),
            full: Some(full),
            trail_id,
        }
    }

    fn require_write(&self) -> Result<&AuditTrailClient<WasmTransactionSigner>> {
        self.full.as_ref().ok_or_else(|| {
            wasm_error(anyhow!(
                "AuditTrailHandle was created from a read-only client; this operation requires AuditTrailClient"
            ))
        })
    }
}

#[wasm_bindgen(js_class = AuditTrailHandle)]
impl WasmAuditTrailHandle {
    /// Loads the full on-chain trail object.
    ///
    /// @remarks
    /// Each call fetches a fresh snapshot from chain state.
    ///
    /// @returns The current {@link OnChainAuditTrail} state of this trail.
    ///
    /// @throws When the trail object cannot be fetched or decoded.
    pub async fn get(&self) -> Result<WasmOnChainAuditTrail> {
        let trail = self.read_only.trail(self.trail_id).get().await.wasm_result()?;
        Ok(trail.into())
    }

    /// Builds a migration transaction for this trail.
    ///
    /// @remarks
    /// Bumps the trail's stored data layout to the current package version. Intended to be called
    /// once after the audit-trail Move package is upgraded.
    ///
    /// Requires the {@link Permission.Migrate} permission.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link Migrate} transaction.
    ///
    /// @throws When the handle was created from a read-only client.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<Migrate>")]
    pub fn migrate(&self) -> Result<WasmTransactionBuilder> {
        let tx = self.require_write()?.trail(self.trail_id).migrate().into_inner();
        Ok(into_transaction_builder(WasmMigrate(tx)))
    }

    /// Builds a delete transaction for this trail.
    ///
    /// @remarks
    /// Deletion additionally requires the trail to be empty (the on-chain call aborts otherwise)
    /// and the configured `deleteTrailLock` to have elapsed.
    ///
    /// Requires the {@link Permission.DeleteAuditTrail} permission.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link DeleteAuditTrail} transaction.
    ///
    /// @throws When the handle was created from a read-only client.
    ///
    /// Emits an {@link AuditTrailDeleted} event on success.
    #[wasm_bindgen(js_name = deleteAuditTrail, unchecked_return_type = "TransactionBuilder<DeleteAuditTrail>")]
    pub fn delete_audit_trail(&self) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .delete_audit_trail()
            .into_inner();
        Ok(into_transaction_builder(WasmDeleteAuditTrail(tx)))
    }

    /// Builds a mutable-metadata update transaction for this trail.
    ///
    /// @remarks
    /// Replaces or clears the trail's `updatableMetadata` field.
    ///
    /// Requires the {@link Permission.UpdateMetadata} permission.
    ///
    /// @param metadata - New value for the trail's `updatableMetadata` field, or `null` to clear
    /// it.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link UpdateMetadata} transaction.
    ///
    /// @throws When the handle was created from a read-only client.
    #[wasm_bindgen(js_name = updateMetadata, unchecked_return_type = "TransactionBuilder<UpdateMetadata>")]
    pub fn update_metadata(&self, metadata: Option<String>) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .update_metadata(metadata)
            .into_inner();
        Ok(into_transaction_builder(WasmUpdateMetadata(tx)))
    }

    /// Returns the record API scoped to this trail.
    ///
    /// @remarks
    /// Use this for record reads, appends, and deletions.
    ///
    /// @returns A {@link TrailRecords} wrapper bound to this trail.
    pub fn records(&self) -> WasmTrailRecords {
        WasmTrailRecords {
            read_only: self.read_only.clone(),
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }

    /// Returns the access-control API scoped to this trail.
    ///
    /// @remarks
    /// Use this for roles, capabilities, and access-policy updates.
    ///
    /// @returns A {@link TrailAccess} wrapper bound to this trail.
    pub fn access(&self) -> WasmTrailAccess {
        WasmTrailAccess {
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }

    /// Returns the locking API scoped to this trail.
    ///
    /// @remarks
    /// Use this for inspecting lock state and updating locking rules.
    ///
    /// @returns A {@link TrailLocking} wrapper bound to this trail.
    pub fn locking(&self) -> WasmTrailLocking {
        WasmTrailLocking {
            read_only: self.read_only.clone(),
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }

    /// Returns the tag-registry API scoped to this trail.
    ///
    /// @remarks
    /// Use this for managing the canonical tag registry that record writes and role-tag
    /// restrictions must reference.
    ///
    /// @returns A {@link TrailTags} wrapper bound to this trail.
    pub fn tags(&self) -> WasmTrailTags {
        WasmTrailTags {
            read_only: self.read_only.clone(),
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }
}
