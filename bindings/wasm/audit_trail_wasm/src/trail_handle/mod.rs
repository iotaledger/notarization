// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Trail-scoped wasm handle wrappers.

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
/// `AuditTrailHandle` keeps one trail ID together with the originating client so all trail-scoped
/// reads and transaction builders can be discovered from a single JS/TS value.
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

    /// Returns the writable client when this handle came from `AuditTrailClient`.
    ///
    /// Throws when the handle was created from `AuditTrailClientReadOnly`.
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
    /// Each call fetches a fresh snapshot from chain state.
    pub async fn get(&self) -> Result<WasmOnChainAuditTrail> {
        let trail = self.read_only.trail(self.trail_id).get().await.wasm_result()?;
        Ok(trail.into())
    }

    /// Builds a migration transaction for this trail.
    ///
    /// Bumps the trail's stored data layout to the current package version. Intended to be called
    /// once after the audit-trail Move package is upgraded. Requires the `Migrate` permission.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<Migrate>")]
    pub fn migrate(&self) -> Result<WasmTransactionBuilder> {
        let tx = self.require_write()?.trail(self.trail_id).migrate().into_inner();
        Ok(into_transaction_builder(WasmMigrate(tx)))
    }

    /// Builds a delete transaction for this trail.
    ///
    /// Requires the `DeleteAuditTrail` permission. Deletion additionally requires the trail to be
    /// empty (the on-chain call aborts otherwise) and the configured `deleteTrailLock` to have
    /// elapsed. Emits an `AuditTrailDeleted` event on success.
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
    /// Replaces or clears the trail's `updatableMetadata` field. Pass `null` to clear the field.
    /// Requires the `UpdateMetadata` permission.
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
    /// Use this for record reads, appends, and deletions.
    pub fn records(&self) -> WasmTrailRecords {
        WasmTrailRecords {
            read_only: self.read_only.clone(),
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }

    /// Returns the access-control API scoped to this trail.
    ///
    /// Use this for roles, capabilities, and access-policy updates.
    pub fn access(&self) -> WasmTrailAccess {
        WasmTrailAccess {
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }

    /// Returns the locking API scoped to this trail.
    ///
    /// Use this for inspecting lock state and updating locking rules.
    pub fn locking(&self) -> WasmTrailLocking {
        WasmTrailLocking {
            read_only: self.read_only.clone(),
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }

    /// Returns the tag-registry API scoped to this trail.
    ///
    /// Use this for managing the canonical tag registry that record writes and role-tag
    /// restrictions must reference.
    pub fn tags(&self) -> WasmTrailTags {
        WasmTrailTags {
            read_only: self.read_only.clone(),
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }
}
