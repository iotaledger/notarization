// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use audit_trails::core::types::{Data as AuditTrailData, RecordInput};
use audit_trails::{AuditTrailClient, AuditTrailClientReadOnly};
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result, WasmResult};
use iota_sdk_types::ObjectId;
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::into_transaction_builder;
use wasm_bindgen::prelude::*;

use crate::trail::{WasmAddRecord, WasmCorrectRecord, WasmDeleteRecord, WasmDeleteRecordsBatch};
use crate::types::{WasmData, WasmPaginatedRecord, WasmRecord};

/// Record API scoped to a specific trail.
///
/// @remarks
/// Builds record-oriented transactions and loads record data from the trail's linked-table
/// storage.
#[derive(Clone)]
#[wasm_bindgen(js_name = TrailRecords, inspectable)]
pub struct WasmTrailRecords {
    pub(crate) read_only: AuditTrailClientReadOnly,
    pub(crate) full: Option<AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectId,
}

impl WasmTrailRecords {
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
    /// @param sequenceNumber - Sequence number of the record to load.
    ///
    /// @returns The record stored at `sequenceNumber`.
    ///
    /// @throws When no record exists at the requested sequence number or the data cannot be
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
    ///
    /// @returns Current record count.
    ///
    /// @throws When the trail object cannot be fetched.
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
    /// @remarks
    /// Traverses the full on-chain linked table and can be expensive for large trails. For
    /// paginated access, use {@link TrailRecords.listPage}.
    ///
    /// @returns Every record in the trail, sorted by ascending sequence number.
    ///
    /// @throws When the trail object cannot be fetched or a record cannot be deserialized.
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
    /// @remarks
    /// Use this as a safety net against unexpectedly large traversals.
    ///
    /// @param maxEntries - Upper bound on the number of records the caller is willing to load.
    ///
    /// @returns Every record in the trail, sorted by ascending sequence number.
    ///
    /// @throws When the trail's linked-table size exceeds `maxEntries`.
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
    /// @param cursor - Sequence-number cursor for the page boundary; pass `null` for the first
    /// page and reuse {@link PaginatedRecord.nextCursor} for subsequent pages.
    /// @param limit - Maximum number of records to return; may not exceed the maximum page size defined in the
    /// Audit Trails Rust crate.
    ///
    /// @returns A {@link PaginatedRecord} carrying the loaded records and pagination metadata.
    ///
    /// @throws When the trail object cannot be fetched, a record cannot be deserialized, or
    /// `limit` exceeds the maximum page size defined in the Audit Trails Rust crate.
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

    /// Loads the current version of a record by following correction links.
    ///
    /// @remarks
    /// Use {@link TrailRecords.get} when you need the exact immutable record stored at a sequence
    /// number. Use `resolveCurrent` when you have an original sequence number and want the latest
    /// correction in that record's replacement chain.
    ///
    /// For example, if record `3` was corrected by record `7`, and record `7` was later corrected
    /// by record `9`, `resolveCurrent(3)` returns record `9`. If the starting record has not been
    /// replaced, this returns the starting record itself.
    ///
    /// @param sequenceNumber - Sequence number to resolve.
    ///
    /// @returns The current record at the end of the correction chain.
    ///
    /// @throws When a record cannot be loaded or the replacement chain is malformed.
    #[wasm_bindgen(js_name = resolveCurrent)]
    pub async fn resolve_current(&self, sequence_number: u64) -> Result<WasmRecord> {
        let record = self
            .read_only
            .trail(self.trail_id)
            .records()
            .resolve_current(sequence_number)
            .await
            .wasm_result()?;
        Ok(record.into())
    }

    /// Builds a record-correction transaction.
    ///
    /// @remarks
    /// Appends a new correction record that supersedes `sequenceNumber` while preserving the
    /// original record. The correction records the sequence number it replaces, and the replaced
    /// record receives a back-pointer to the new correction so `resolveCurrent` can follow the
    /// replacement chain.
    ///
    /// Tagged corrections require the correction tag to exist in the trail registry and the
    /// supplied capability's role to allow both the replaced record's tag, when present, and the
    /// correction record's tag, when present. The transaction aborts on-chain when the package
    /// version is incompatible, the capability is invalid, the trail is write-locked, the target
    /// record does not exist, the target record was already replaced, or tag authorization fails.
    ///
    /// Requires the {@link Permission.CorrectRecord} permission.
    ///
    /// @param sequenceNumber - Sequence number of the record to correct.
    /// @param data - Replacement record payload.
    /// @param metadata - Optional application-defined metadata stored alongside the correction.
    /// @param tag - Optional trail-owned tag attached to the correction.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link CorrectRecord} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
    ///
    /// Emits a {@link RecordAdded} event on success.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<CorrectRecord>")]
    pub fn correct(
        &self,
        sequence_number: u64,
        data: WasmData,
        metadata: Option<String>,
        tag: Option<String>,
    ) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .records()
            .correct(
                sequence_number,
                RecordInput::new(AuditTrailData::from(data), metadata, tag),
            )
            .into_inner();
        Ok(into_transaction_builder(WasmCorrectRecord(tx)))
    }

    /// Builds a record-add transaction.
    ///
    /// @remarks
    /// Records are appended sequentially with auto-assigned, monotonically increasing sequence
    /// numbers that are never reused. While the trail's `writeLock` is active the on-chain call
    /// aborts. When `tag` is set, it must already exist in the trail's record-tag registry and the
    /// supplied capability's role must allow that tag.
    ///
    /// Requires the {@link Permission.AddRecord} permission.
    ///
    /// @param data - {@link Data} payload of the new record.
    /// @param metadata - Optional application-defined metadata stored alongside the record.
    /// @param tag - Optional trail-owned tag attached to the record.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link AddRecord} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
    ///
    /// Emits a {@link RecordAdded} event on success.
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
    /// @remarks
    /// The on-chain call aborts when no record exists at `sequenceNumber` or while the configured
    /// delete-record window still protects it. When the record carries a tag, the supplied
    /// capability's role must allow that tag.
    ///
    /// Requires the {@link Permission.DeleteRecord} permission.
    ///
    /// @param sequenceNumber - Sequence number of the record to delete.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link DeleteRecord} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
    ///
    /// Emits a {@link RecordDeleted} event on success.
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
    /// @remarks
    /// Walks the trail from the front and silently skips records still inside the delete-record
    /// window or whose tag the capability does not allow, deleting up to `limit` eligible records
    /// in trail order. The set of locked records is fixed at the start of the on-chain call:
    /// count-based windows protect the last `count` records present when the call begins, and
    /// time-based windows are evaluated against the clock timestamp captured at that point.
    /// Running this batch with `limit` therefore yields the same final trail state as deleting
    /// each eligible sequence number one at a time, provided the locking configuration is not
    /// mutated and no records are appended between calls.
    ///
    /// `limit` caps the number of records actually deleted, not the number of records inspected.
    /// Ineligible records at the front of the trail are silently walked past without counting
    /// toward `limit`, so more than `limit` records may be visited before `limit` deletions
    /// accumulate.
    ///
    /// Requires the {@link Permission.DeleteAllRecords} permission.
    ///
    /// @param limit - Maximum number of records to delete in this batch.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link DeleteRecordsBatch} transaction;
    /// when applied it resolves to the sequence numbers of the records deleted in this batch, in
    /// deletion order — at most `limit` entries, possibly fewer.
    ///
    /// @throws When the wrapper was created from a read-only client.
    ///
    /// Emits one {@link RecordDeleted} event per deletion.
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
