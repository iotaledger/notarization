// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Transaction payloads for record writes and deletions.
//!
//! These types cache the generated programmable transaction, delegate PTB construction to
//! [`super::operations::RecordsOps`], and decode record events into typed Rust outputs.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::{IotaTransactionBlockEffects, IotaTransactionBlockEvents};
use iota_interaction::types::base_types::IotaAddress;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_sdk_types::ObjectId;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::operations::RecordsOps;
use crate::core::internal::tx;
use crate::core::types::{Data, Event, RecordAdded, RecordDeleted, RecordInput};
use crate::error::Error;

// ===== AddRecord =====

/// Transaction that appends a record to a trail.
///
/// Requires the `AddRecord` permission. Tagged writes additionally require the tag to exist in the trail
/// registry and a capability whose role explicitly allows that tag; otherwise the Move package aborts with
/// `ERecordTagNotDefined` or `ERecordTagNotAllowed`. The package also aborts with `ETrailWriteLocked` while
/// the configured `write_lock` is active. On success the new record is stored at the trail's current
/// monotonic sequence number (which never decrements, even after deletions) and a `RecordAdded` event is
/// emitted.
#[derive(Debug, Clone)]
pub struct AddRecord {
    /// Trail object ID that will receive the record.
    pub trail_id: ObjectId,
    /// Address authorizing the write.
    pub owner: IotaAddress,
    /// Record payload to append.
    pub data: Data,
    /// Optional application-defined metadata.
    pub metadata: Option<String>,
    /// Optional trail-owned tag to attach to the record.
    pub tag: Option<String>,
    /// Explicit capability to use instead of auto-selecting one from the owner's wallet.
    pub selected_capability_id: Option<ObjectId>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl AddRecord {
    /// Creates an `AddRecord` transaction builder payload.
    pub fn new(
        trail_id: ObjectId,
        owner: IotaAddress,
        data: Data,
        metadata: Option<String>,
        tag: Option<String>,
        selected_capability_id: Option<ObjectId>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            data,
            metadata,
            tag,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RecordsOps::add_record(
            client,
            self.trail_id,
            self.owner,
            RecordInput::new(self.data.clone(), self.metadata.clone(), self.tag.clone()),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for AddRecord {
    type Error = Error;
    type Output = RecordAdded;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| serde_json::from_value::<Event<RecordAdded>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("RecordAdded event not found".to_string()))?;

        Ok(event.data)
    }

    async fn apply<C>(self, effects: &mut IotaTransactionBlockEffects, client: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::apply_with_events(self, effects, client).await
    }
}

// ===== CorrectRecord =====

/// Transaction that appends a correction record to a trail.
///
/// The original record remains immutable. The correction is appended at the trail's next sequence number with
/// a correction tracker whose `replaces` set contains the corrected sequence number, and the corrected record
/// receives a back-pointer to the new correction.
///
/// Requires the `CorrectRecord` permission. Tagged corrections require the correction tag to exist in the trail
/// registry and the capability's role to allow both the replaced record's tag, when present, and the correction
/// tag, when present. The Move call aborts when the trail package version is incompatible, the capability is
/// invalid, the trail is write-locked, the target record does not exist, the target record was already replaced,
/// or tag authorization fails. On success the correction is stored at the trail's current monotonic sequence
/// number and a `RecordAdded` event is emitted.
#[derive(Debug, Clone)]
pub struct CorrectRecord {
    /// Trail object ID that will receive the correction.
    pub trail_id: ObjectId,
    /// Address authorizing the correction.
    pub owner: IotaAddress,
    /// Sequence number of the record being corrected.
    pub sequence_number: u64,
    /// Correction payload to append.
    pub data: Data,
    /// Optional application-defined metadata.
    pub metadata: Option<String>,
    /// Optional trail-owned tag to attach to the correction record.
    pub tag: Option<String>,
    /// Explicit capability to use instead of auto-selecting one from the owner's wallet.
    pub selected_capability_id: Option<ObjectId>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl CorrectRecord {
    /// Creates a `CorrectRecord` transaction builder payload.
    ///
    /// The resulting transaction appends a correction record for `sequence_number` and carries the same
    /// authorization, write-lock, record-existence, already-replaced, tag-definition, and tag-authorization
    /// requirements as the Move `correct_record` entry point.
    pub fn new(
        trail_id: ObjectId,
        owner: IotaAddress,
        sequence_number: u64,
        data: Data,
        metadata: Option<String>,
        tag: Option<String>,
        selected_capability_id: Option<ObjectId>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            sequence_number,
            data,
            metadata,
            tag,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RecordsOps::correct_record(
            client,
            self.trail_id,
            self.owner,
            self.sequence_number,
            RecordInput::new(self.data.clone(), self.metadata.clone(), self.tag.clone()),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for CorrectRecord {
    type Error = Error;
    type Output = RecordAdded;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| serde_json::from_value::<Event<RecordAdded>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("RecordAdded event not found".to_string()))?;

        Ok(event.data)
    }

    async fn apply<C>(self, effects: &mut IotaTransactionBlockEffects, client: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::apply_with_events(self, effects, client).await
    }
}

// ===== DeleteRecord =====

/// Transaction that deletes a single record.
///
/// Requires the `DeleteRecord` permission. The Move package aborts with `ERecordNotFound` when no record
/// exists at `sequence_number` and with `ERecordLocked` while the configured delete-record window still
/// protects the record. Tag-aware authorization additionally applies: if the record carries a tag, the
/// supplied capability's role must allow that tag.
///
/// On success a `RecordDeleted` event is emitted.
#[derive(Debug, Clone)]
pub struct DeleteRecord {
    /// Trail object ID containing the record.
    pub trail_id: ObjectId,
    /// Address authorizing the deletion.
    pub owner: IotaAddress,
    /// Sequence number of the record to delete.
    pub sequence_number: u64,
    /// Explicit capability to use instead of auto-selecting one from the owner's wallet.
    pub selected_capability_id: Option<ObjectId>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DeleteRecord {
    /// Creates a `DeleteRecord` transaction builder payload.
    pub fn new(
        trail_id: ObjectId,
        owner: IotaAddress,
        sequence_number: u64,
        selected_capability_id: Option<ObjectId>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            sequence_number,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RecordsOps::delete_record(
            client,
            self.trail_id,
            self.owner,
            self.sequence_number,
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for DeleteRecord {
    type Error = Error;
    type Output = RecordDeleted;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        mut self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let event = events
            .data
            .iter()
            .find_map(|data| serde_json::from_value::<Event<RecordDeleted>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("RecordDeleted event not found".to_string()))?;

        Ok(event.data)
    }

    async fn apply<C>(self, effects: &mut IotaTransactionBlockEffects, client: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::apply_with_events(self, effects, client).await
    }
}

// ===== DeleteRecordsBatch =====

/// Transaction that deletes multiple records in a batch operation.
///
/// Requires the `DeleteAllRecords` permission. The Move entry point walks the trail from the front,
/// silently skips records still inside the delete-record window or outside the capability's allowed tag set,
/// and deletes up to `limit` eligible records in trail order.
///
/// On success a `RecordDeleted` event is emitted per deletion.
///
/// `limit` caps the number of records actually deleted, not the number of records inspected. Ineligible
/// records at the front of the trail are silently walked past without counting toward `limit`, so more
/// than `limit` records may be visited before `limit` deletions accumulate.
///
/// Lock state — both count-based
/// and time-based — is evaluated against the trail snapshot and clock timestamp captured at the start of the
/// call, so the deletable set is stable for the batch's duration. The Rust implementation mirrors the Move
/// output by collecting the matching `RecordDeleted` events in deletion order; the returned vector may be
/// shorter than `limit` (or empty) and that is not an error.
#[derive(Debug, Clone)]
pub struct DeleteRecordsBatch {
    /// Trail object ID containing the records.
    pub trail_id: ObjectId,
    /// Address authorizing the deletion.
    pub owner: IotaAddress,
    /// Maximum number of records to delete in this batch.
    pub limit: u64,
    /// Explicit capability to use instead of auto-selecting one from the owner's wallet.
    pub selected_capability_id: Option<ObjectId>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DeleteRecordsBatch {
    /// Creates a `DeleteRecordsBatch` transaction builder payload.
    pub fn new(trail_id: ObjectId, owner: IotaAddress, limit: u64, selected_capability_id: Option<ObjectId>) -> Self {
        Self {
            trail_id,
            owner,
            limit,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        RecordsOps::delete_records_batch(
            client,
            self.trail_id,
            self.owner,
            self.limit,
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for DeleteRecordsBatch {
    type Error = Error;
    type Output = Vec<u64>;

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply_with_events<C>(
        self,
        _: &mut IotaTransactionBlockEffects,
        events: &mut IotaTransactionBlockEvents,
        _: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let deleted = events
            .data
            .iter()
            .filter_map(|data| serde_json::from_value::<Event<RecordDeleted>>(data.parsed_json.clone()).ok())
            .map(|event| event.data.sequence_number)
            .collect();

        Ok(deleted)
    }

    async fn apply<C>(self, effects: &mut IotaTransactionBlockEffects, client: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::apply_with_events(self, effects, client).await
    }
}
