// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;
use serde::de::DeserializeOwned;

use crate::core::locking::TrailLocking;
use crate::core::records::TrailRecords;
use crate::core::roles::TrailRoles;
use crate::core::types::{Data, OnChainAuditTrail};
use crate::error::Error;

mod operations;
mod transactions;

pub use transactions::{AddRecordTag, DeleteAuditTrail, Migrate, RemoveRecordTag, SetRecordTags, UpdateMetadata};

/// Marker trait for read-only audit trail clients.
#[doc(hidden)]
#[async_trait::async_trait]
pub trait AuditTrailReadOnly: CoreClientReadOnly + OptionalSync {
    async fn execute_read_only_transaction<T: DeserializeOwned>(&self, tx: ProgrammableTransaction)
    -> Result<T, Error>;
}

/// Marker trait for full (read-write) audit trail clients.
#[doc(hidden)]
pub trait AuditTrailFull: AuditTrailReadOnly {}

/// A typed handle bound to a specific audit trail and client.
#[derive(Debug, Clone)]
pub struct AuditTrailHandle<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
}

impl<'a, C> AuditTrailHandle<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self { client, trail_id }
    }

    /// Loads the full on-chain audit trail object.
    pub async fn get(&self) -> Result<OnChainAuditTrail, Error>
    where
        C: AuditTrailReadOnly,
    {
        crate::core::operations::get_audit_trail(self.trail_id, self.client).await
    }

    /// Updates the trail's updatable metadata.
    pub fn update_metadata<S>(&self, metadata: Option<String>) -> TransactionBuilder<UpdateMetadata>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(UpdateMetadata::new(self.trail_id, owner, metadata))
    }

    /// Migrates the trail to the latest package version.
    pub fn migrate<S>(&self) -> TransactionBuilder<Migrate>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(Migrate::new(self.trail_id, owner))
    }

    /// Deletes the audit trail object.
    ///
    /// The trail must be empty before deletion.
    pub fn delete_audit_trail<S>(&self) -> TransactionBuilder<DeleteAuditTrail>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(DeleteAuditTrail::new(self.trail_id, owner))
    }

    /// Adds a tag to the trail-owned record-tag registry.
    pub fn add_record_tag<S>(&self, tag: impl Into<String>) -> TransactionBuilder<AddRecordTag>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(AddRecordTag::new(self.trail_id, owner, tag.into()))
    }

    /// Removes a tag from the trail-owned record-tag registry.
    pub fn remove_record_tag<S>(&self, tag: impl Into<String>) -> TransactionBuilder<RemoveRecordTag>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(RemoveRecordTag::new(self.trail_id, owner, tag.into()))
    }

    /// Replaces the entire trail-owned record-tag registry.
    pub fn set_record_tags<S, I, T>(&self, tags: I) -> TransactionBuilder<SetRecordTags>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(SetRecordTags::new(
            self.trail_id,
            owner,
            tags.into_iter().map(Into::into).collect(),
        ))
    }

    pub fn records(&self) -> TrailRecords<'a, C, Data> {
        TrailRecords::new(self.client, self.trail_id)
    }

    pub fn locking(&self) -> TrailLocking<'a, C> {
        TrailLocking::new(self.client, self.trail_id)
    }

    pub fn roles(&self) -> TrailRoles<'a, C> {
        TrailRoles::new(self.client, self.trail_id)
    }
}
