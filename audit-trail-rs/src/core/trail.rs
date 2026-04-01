// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! High-level trail handle types and trail-scoped transactions.

use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;
use serde::de::DeserializeOwned;

use crate::core::access::TrailAccess;
use crate::core::internal::trail as trail_reader;
use crate::core::locking::TrailLocking;
use crate::core::records::TrailRecords;
use crate::core::tags::TrailTags;
use crate::core::types::{Data, OnChainAuditTrail};
use crate::error::Error;

mod operations;
mod transactions;

pub use transactions::{DeleteAuditTrail, Migrate, UpdateMetadata};

/// Marker trait for read-only audit trail clients.
#[doc(hidden)]
#[cfg_attr(not(feature = "send-sync"), async_trait::async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait::async_trait)]
pub trait AuditTrailReadOnly: CoreClientReadOnly + OptionalSync {
    /// Executes a read-only programmable transaction and decodes the first return value.
    async fn execute_read_only_transaction<T: DeserializeOwned>(&self, tx: ProgrammableTransaction)
    -> Result<T, Error>;
}

/// Marker trait for full (read-write) audit trail clients.
#[doc(hidden)]
pub trait AuditTrailFull: AuditTrailReadOnly {}

/// A typed handle bound to a specific audit trail and client.
///
/// `AuditTrailHandle` is the main trail-scoped entry point. It keeps the trail ID together with
/// the client so that record, locking, access-control, tag, and metadata operations can all hang
/// off one typed value.
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
    ///
    /// Each call fetches a fresh snapshot from chain state.
    pub async fn get(&self) -> Result<OnChainAuditTrail, Error>
    where
        C: AuditTrailReadOnly,
    {
        trail_reader::get_audit_trail(self.trail_id, self.client).await
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

    /// Returns the record API scoped to this trail.
    ///
    /// Use this for record reads and record-oriented transaction builders.
    pub fn records(&self) -> TrailRecords<'a, C, Data> {
        TrailRecords::new(self.client, self.trail_id)
    }

    /// Returns the locking API scoped to this trail.
    ///
    /// Use this for checking and updating trail-level locking rules.
    pub fn locking(&self) -> TrailLocking<'a, C> {
        TrailLocking::new(self.client, self.trail_id)
    }

    /// Returns the access-control API scoped to this trail.
    ///
    /// Use this for roles, capabilities, and access-policy updates.
    pub fn access(&self) -> TrailAccess<'a, C> {
        TrailAccess::new(self.client, self.trail_id)
    }

    /// Returns the tag-registry API scoped to this trail.
    ///
    /// Use this for managing the set of tags available to records in this trail.
    pub fn tags(&self) -> TrailTags<'a, C> {
        TrailTags::new(self.client, self.trail_id)
    }
}
