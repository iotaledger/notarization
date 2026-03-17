// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod access;
mod locking;
mod records;

use anyhow::anyhow;
use audit_trails::{AuditTrailClient, AuditTrailClientReadOnly};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result, WasmResult};
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::into_transaction_builder;
use wasm_bindgen::prelude::*;

use crate::trail::{WasmDeleteAuditTrail, WasmMigrate, WasmOnChainAuditTrail, WasmUpdateMetadata};

pub(crate) use access::WasmTrailAccess;
pub(crate) use locking::WasmTrailLocking;
pub(crate) use records::WasmTrailRecords;

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
    pub async fn get(&self) -> Result<WasmOnChainAuditTrail> {
        let trail = self.read_only.trail(self.trail_id).get().await.wasm_result()?;
        Ok(trail.into())
    }

    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<Migrate>")]
    pub fn migrate(&self) -> Result<WasmTransactionBuilder> {
        let tx = self.require_write()?.trail(self.trail_id).migrate().into_inner();
        Ok(into_transaction_builder(WasmMigrate(tx)))
    }

    #[wasm_bindgen(js_name = deleteAuditTrail, unchecked_return_type = "TransactionBuilder<DeleteAuditTrail>")]
    pub fn delete_audit_trail(&self) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .delete_audit_trail()
            .into_inner();
        Ok(into_transaction_builder(WasmDeleteAuditTrail(tx)))
    }

    #[wasm_bindgen(js_name = updateMetadata, unchecked_return_type = "TransactionBuilder<UpdateMetadata>")]
    pub fn update_metadata(&self, metadata: Option<String>) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .update_metadata(metadata)
            .into_inner();
        Ok(into_transaction_builder(WasmUpdateMetadata(tx)))
    }

    pub fn records(&self) -> WasmTrailRecords {
        WasmTrailRecords {
            read_only: self.read_only.clone(),
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }

    pub fn access(&self) -> WasmTrailAccess {
        WasmTrailAccess {
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }

    pub fn locking(&self) -> WasmTrailLocking {
        WasmTrailLocking {
            read_only: self.read_only.clone(),
            full: self.full.clone(),
            trail_id: self.trail_id,
        }
    }
}
