// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result};
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::into_transaction_builder;
use wasm_bindgen::prelude::*;

use crate::trail::{WasmAddRecord, WasmDeleteRecord, WasmDeleteRecordsBatch};
use crate::types::{WasmPaginatedRecord, WasmRecord};
use crate::audit_trails_wasm_result;

#[derive(Clone)]
#[wasm_bindgen(js_name = TrailRecords, inspectable)]
pub struct WasmTrailRecords {
    pub(crate) read_only: audit_trails::AuditTrailClientReadOnly,
    pub(crate) full: Option<audit_trails::AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
}

impl WasmTrailRecords {
    fn full_client(&self) -> Result<&audit_trails::AuditTrailClient<WasmTransactionSigner>> {
        self.full.as_ref().ok_or_else(|| {
            wasm_error(anyhow!(
                "TrailRecords was created from a read-only client; this operation requires AuditTrailClient"
            ))
        })
    }
}

#[wasm_bindgen(js_class = TrailRecords)]
impl WasmTrailRecords {
    pub async fn get(&self, sequence_number: u64) -> Result<WasmRecord> {
        let record = audit_trails_wasm_result(
            self
            .read_only
            .trail(self.trail_id)
            .records()
            .get(sequence_number)
            .await,
        )?;
        Ok(record.into())
    }

    #[wasm_bindgen(js_name = recordCount)]
    pub async fn record_count(&self) -> Result<u64> {
        audit_trails_wasm_result(self.read_only.trail(self.trail_id).records().record_count().await)
    }

    pub async fn list(&self) -> Result<Vec<WasmRecord>> {
        let mut records: Vec<_> =
            audit_trails_wasm_result(self.read_only.trail(self.trail_id).records().list().await)?
                .into_iter()
                .collect();
        records.sort_unstable_by_key(|(sequence_number, _)| *sequence_number);
        Ok(records.into_iter().map(|(_, record)| record.into()).collect())
    }

    #[wasm_bindgen(js_name = listPage)]
    pub async fn list_page(&self, cursor: Option<u64>, limit: usize) -> Result<WasmPaginatedRecord> {
        let page = audit_trails_wasm_result(self.read_only.trail(self.trail_id).records().list_page(cursor, limit).await)?;
        Ok(page.into())
    }

    #[wasm_bindgen(js_name = addString, unchecked_return_type = "TransactionBuilder<AddRecord>")]
    pub fn add_string(&self, data: String, metadata: Option<String>) -> Result<WasmTransactionBuilder> {
        let tx = self
            .full_client()?
            .trail(self.trail_id)
            .records()
            .add(audit_trails::core::types::Data::text(data), metadata)
            .into_inner();
        Ok(into_transaction_builder(WasmAddRecord(tx)))
    }

    #[wasm_bindgen(js_name = addBytes, unchecked_return_type = "TransactionBuilder<AddRecord>")]
    pub fn add_bytes(
        &self,
        data: js_sys::Uint8Array,
        metadata: Option<String>,
    ) -> Result<WasmTransactionBuilder> {
        let tx = self
            .full_client()?
            .trail(self.trail_id)
            .records()
            .add(audit_trails::core::types::Data::bytes(data.to_vec()), metadata)
            .into_inner();
        Ok(into_transaction_builder(WasmAddRecord(tx)))
    }

    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<DeleteRecord>")]
    pub fn delete(&self, sequence_number: u64) -> Result<WasmTransactionBuilder> {
        let tx = self
            .full_client()?
            .trail(self.trail_id)
            .records()
            .delete(sequence_number)
            .into_inner();
        Ok(into_transaction_builder(WasmDeleteRecord(tx)))
    }

    #[wasm_bindgen(js_name = deleteBatch, unchecked_return_type = "TransactionBuilder<DeleteRecordsBatch>")]
    pub fn delete_batch(&self, limit: u64) -> Result<WasmTransactionBuilder> {
        let tx = self
            .full_client()?
            .trail(self.trail_id)
            .records()
            .delete_records_batch(limit)
            .into_inner();
        Ok(into_transaction_builder(WasmDeleteRecordsBatch(tx)))
    }
}
