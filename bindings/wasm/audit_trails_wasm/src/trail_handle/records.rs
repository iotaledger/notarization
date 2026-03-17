// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use audit_trails::core::types::Data as AuditTrailData;
use audit_trails::{AuditTrailClient, AuditTrailClientReadOnly};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result, WasmResult};
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::into_transaction_builder;
use wasm_bindgen::prelude::*;

use crate::trail::{WasmAddRecord, WasmDeleteRecord, WasmDeleteRecordsBatch};
use crate::types::{WasmData, WasmEmpty, WasmPaginatedRecord, WasmRecord};

#[derive(Clone)]
#[wasm_bindgen(js_name = TrailRecords, inspectable)]
pub struct WasmTrailRecords {
    pub(crate) read_only: AuditTrailClientReadOnly,
    pub(crate) full: Option<AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
}

impl WasmTrailRecords {
    /// Returns the writable client for record mutations.
    ///
    /// Throws when this wrapper was created from `AuditTrailClientReadOnly`.
    fn require_write(&self) -> Result<&AuditTrailClient<WasmTransactionSigner>> {
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
        let record = self
            .read_only
            .trail(self.trail_id)
            .records()
            .get(sequence_number)
            .await
            .wasm_result()?;
        Ok(record.into())
    }

    #[wasm_bindgen(js_name = recordCount)]
    pub async fn record_count(&self) -> Result<u64> {
        self.read_only
            .trail(self.trail_id)
            .records()
            .record_count()
            .await
            .wasm_result()
    }

    pub async fn list(&self) -> Result<Vec<WasmRecord>> {
        let mut records: Vec<_> = self
            .read_only
            .trail(self.trail_id)
            .records()
            .list()
            .await
            .wasm_result()?
            .into_iter()
            .collect();
        records.sort_unstable_by_key(|(sequence_number, _)| *sequence_number);
        Ok(records.into_iter().map(|(_, record)| record.into()).collect())
    }

    #[wasm_bindgen(js_name = listWithLimit)]
    pub async fn list_with_limit(&self, max_entries: usize) -> Result<Vec<WasmRecord>> {
        let mut records: Vec<_> = self
            .read_only
            .trail(self.trail_id)
            .records()
            .list_with_limit(max_entries)
            .await
            .wasm_result()?
            .into_iter()
            .collect();
        records.sort_unstable_by_key(|(sequence_number, _)| *sequence_number);
        Ok(records.into_iter().map(|(_, record)| record.into()).collect())
    }

    #[wasm_bindgen(js_name = listPage)]
    pub async fn list_page(&self, cursor: Option<u64>, limit: usize) -> Result<WasmPaginatedRecord> {
        let page = self
            .read_only
            .trail(self.trail_id)
            .records()
            .list_page(cursor, limit)
            .await
            .wasm_result()?;
        Ok(page.into())
    }

    pub async fn correct(&self, replaces: Vec<u64>, data: WasmData, metadata: Option<String>) -> Result<WasmEmpty> {
        self.require_write()?
            .trail(self.trail_id)
            .records()
            .correct(replaces, data.into(), metadata)
            .await
            .wasm_result()?;
        Ok(WasmEmpty)
    }

    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<AddRecord>")]
    pub fn add(&self, data: WasmData, metadata: Option<String>) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .records()
            .add(AuditTrailData::from(data), metadata)
            .into_inner();
        Ok(into_transaction_builder(WasmAddRecord(tx)))
    }

    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<DeleteRecord>")]
    pub fn delete(&self, sequence_number: u64) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .records()
            .delete(sequence_number)
            .into_inner();
        Ok(into_transaction_builder(WasmDeleteRecord(tx)))
    }

    #[wasm_bindgen(js_name = deleteBatch, unchecked_return_type = "TransactionBuilder<DeleteRecordsBatch>")]
    pub fn delete_batch(&self, limit: u64) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .records()
            .delete_records_batch(limit)
            .into_inner();
        Ok(into_transaction_builder(WasmDeleteRecordsBatch(tx)))
    }
}
