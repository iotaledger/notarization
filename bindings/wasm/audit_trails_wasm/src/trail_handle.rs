// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result};
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::into_transaction_builder;
use wasm_bindgen::prelude::*;

use crate::trail::{WasmOnChainAuditTrail, WasmUpdateMetadata};
use crate::trail_records::WasmTrailRecords;
use crate::audit_trails_wasm_result;

#[derive(Clone)]
#[wasm_bindgen(js_name = AuditTrailHandle, inspectable)]
pub struct WasmAuditTrailHandle {
    pub(crate) read_only: audit_trails::AuditTrailClientReadOnly,
    pub(crate) full: Option<audit_trails::AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
}

impl WasmAuditTrailHandle {
    pub(crate) fn from_read_only(read_only: audit_trails::AuditTrailClientReadOnly, trail_id: ObjectID) -> Self {
        Self {
            read_only,
            full: None,
            trail_id,
        }
    }

    pub(crate) fn from_full(full: audit_trails::AuditTrailClient<WasmTransactionSigner>, trail_id: ObjectID) -> Self {
        Self {
            read_only: full.read_only().clone(),
            full: Some(full),
            trail_id,
        }
    }

    fn full_client(&self) -> Result<&audit_trails::AuditTrailClient<WasmTransactionSigner>> {
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
        let trail = audit_trails_wasm_result(self.read_only.trail(self.trail_id).get().await)?;
        Ok(trail.into())
    }

    #[wasm_bindgen(js_name = updateMetadata, unchecked_return_type = "TransactionBuilder<UpdateMetadata>")]
    pub fn update_metadata(&self, metadata: Option<String>) -> Result<WasmTransactionBuilder> {
        let tx = self
            .full_client()?
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
}
