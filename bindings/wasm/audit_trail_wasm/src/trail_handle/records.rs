// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use audit_trail::core::types::Data as AuditTrailData;
use audit_trail::{AuditTrailClient, AuditTrailClientReadOnly};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result, WasmResult};
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::into_transaction_builder;
use wasm_bindgen::prelude::*;

use crate::trail::{WasmAddRecord, WasmDeleteRecord, WasmDeleteRecordsBatch};
use crate::types::{WasmData, WasmEmpty, WasmPaginatedRecord, WasmRecord};

/// Record API scoped to a specific trail.
///
/// Builds record-oriented transactions and loads record data from the trail's linked-table
/// storage.
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
    /// Loads one record by sequence number.
    ///
    /// Throws when no record exists at the requested sequence number or the data cannot be
    /// deserialized.
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

    /// Returns the number of records currently stored in the trail.
    #[wasm_bindgen(js_name = recordCount)]
    pub async fn record_count(&self) -> Result<u64> {
        self.read_only
            .trail(self.trail_id)
            .records()
            .record_count()
            .await
            .wasm_result()
    }

    /// Lists all records in sequence-number order.
    ///
    /// Traverses the full on-chain linked table and can be expensive for large trails. For
    /// paginated access, use [`listPage`](Self::list_page).
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

    /// Lists all records while enforcing a maximum number of entries.
    ///
    /// Throws when the trail's linked-table size exceeds `maxEntries`. Use this as a safety net
    /// against unexpectedly large traversals.
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

    /// Loads one page of records starting at `cursor`.
    ///
    /// Pass `null` for the first page; reuse the returned `nextCursor` for subsequent pages.
    /// `limit` may not exceed the SDK-side maximum page size.
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

    /// Executes the correction helper for a record payload.
    ///
    /// Placeholder for a future correction helper — currently always throws because the
    /// underlying Rust implementation is not yet wired up.
    pub async fn correct(&self, replaces: Vec<u64>, data: WasmData, metadata: Option<String>) -> Result<WasmEmpty> {
        self.require_write()?
            .trail(self.trail_id)
            .records()
            .correct(replaces, data.into(), metadata)
            .await
            .wasm_result()?;
        Ok(WasmEmpty)
    }

    /// Builds a record-add transaction.
    ///
    /// Records are appended sequentially with auto-assigned, monotonically increasing sequence
    /// numbers that are never reused. While the trail's `writeLock` is active the on-chain call
    /// aborts. When `tag` is set, it must already exist in the trail's record-tag registry and
    /// the supplied capability's role must allow that tag. Requires the `AddRecord` permission.
    /// Emits a `RecordAdded` event on success.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<AddRecord>")]
    pub fn add(&self, data: WasmData, metadata: Option<String>, tag: Option<String>) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .records()
            .add(AuditTrailData::from(data), metadata, tag)
            .into_inner();
        Ok(into_transaction_builder(WasmAddRecord(tx)))
    }

    /// Builds a single-record delete transaction.
    ///
    /// The on-chain call aborts when no record exists at `sequenceNumber` or while the
    /// configured delete-record window still protects it. When the record carries a tag, the
    /// supplied capability's role must allow that tag. Requires the `DeleteRecord` permission.
    /// Emits a `RecordDeleted` event on success.
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

    /// Builds a batched record-delete transaction.
    ///
    /// Walks the trail from the front and silently skips records still inside the delete-record
    /// window, deleting up to `limit` unlocked records in trail order. Tag-aware authorization
    /// applies to every record actually deleted. Requires the `DeleteAllRecords` permission.
    /// Resolves to the sequence numbers of the records deleted in this batch, in deletion order
    /// — at most `limit` entries, possibly fewer. Emits one `RecordDeleted` event per deletion.
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
