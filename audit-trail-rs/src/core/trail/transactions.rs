// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Transaction payloads for trail-level metadata, migration, and deletion operations.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::{IotaTransactionBlockEffects, IotaTransactionBlockEvents};
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::operations::TrailOps;
use crate::core::types::{AuditTrailDeleted, Event};
use crate::error::Error;

/// Transaction that migrates a trail to the latest package version supported by this crate.
///
/// This requires the `Migrate` permission on the supplied capability and succeeds only when the on-chain
/// package version is *strictly less* than the current supported version. Otherwise the Move package aborts
/// with `EPackageVersionMismatch`.
#[derive(Debug, Clone)]
pub struct Migrate {
    trail_id: ObjectID,
    owner: IotaAddress,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl Migrate {
    /// Creates a `Migrate` transaction builder payload.
    pub fn new(trail_id: ObjectID, owner: IotaAddress, selected_capability_id: Option<ObjectID>) -> Self {
        Self {
            trail_id,
            owner,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        TrailOps::migrate(client, self.trail_id, self.owner, self.selected_capability_id).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for Migrate {
    type Error = Error;
    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply<C>(self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}

/// Transaction that updates mutable trail metadata.
///
/// Requires the `UpdateMetadata` permission on the supplied capability. Passing `None` clears the mutable
/// metadata field on-chain.
#[derive(Debug, Clone)]
pub struct UpdateMetadata {
    trail_id: ObjectID,
    owner: IotaAddress,
    metadata: Option<String>,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateMetadata {
    /// Creates an `UpdateMetadata` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        metadata: Option<String>,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            metadata,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        TrailOps::update_metadata(
            client,
            self.trail_id,
            self.owner,
            self.metadata.clone(),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for UpdateMetadata {
    type Error = Error;
    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply<C>(self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}

/// Transaction that deletes an empty trail.
///
/// Requires the `DeleteAuditTrail` permission. The Move package additionally aborts with
/// `ETrailNotEmpty` while any records remain in the trail and with `ETrailDeleteLocked` while the
/// configured `delete_trail_lock` is still active. On success an `AuditTrailDeleted` event is emitted.
#[derive(Debug, Clone)]
pub struct DeleteAuditTrail {
    trail_id: ObjectID,
    owner: IotaAddress,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DeleteAuditTrail {
    /// Creates a `DeleteAuditTrail` transaction builder payload.
    pub fn new(trail_id: ObjectID, owner: IotaAddress, selected_capability_id: Option<ObjectID>) -> Self {
        Self {
            trail_id,
            owner,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        TrailOps::delete_audit_trail(client, self.trail_id, self.owner, self.selected_capability_id).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for DeleteAuditTrail {
    type Error = Error;
    type Output = AuditTrailDeleted;

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
        let event = events
            .data
            .iter()
            .find_map(|data| serde_json::from_value::<Event<AuditTrailDeleted>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("Expected AuditTrailDeleted event not found".to_string()))?;

        Ok(event.data)
    }

    async fn apply<C>(self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}
