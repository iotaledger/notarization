// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! High-level trail handles and trail-scoped transactions.

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

/// Marker trait for read-only audit-trail clients.
#[doc(hidden)]
#[cfg_attr(not(feature = "send-sync"), async_trait::async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait::async_trait)]
pub trait AuditTrailReadOnly: CoreClientReadOnly + OptionalSync {
    /// Executes a read-only programmable transaction and decodes the first return value.
    async fn execute_read_only_transaction<T: DeserializeOwned>(&self, tx: ProgrammableTransaction)
    -> Result<T, Error>;
}

/// Marker trait for full audit-trail clients.
#[doc(hidden)]
pub trait AuditTrailFull: AuditTrailReadOnly {}

/// A typed handle bound to one trail ID and one client.
///
/// This is the main trail-scoped entry point. It keeps the trail identity together with the client so record,
/// locking, access, tag, migration, and metadata operations all share one typed handle.
#[derive(Debug, Clone)]
pub struct AuditTrailHandle<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
    pub(crate) selected_capability_id: Option<ObjectID>,
}

impl<'a, C> AuditTrailHandle<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self {
            client,
            trail_id,
            selected_capability_id: None,
        }
    }

    /// Uses the provided capability as the auth capability for subsequent write operations.
    pub fn using_capability(mut self, capability_id: ObjectID) -> Self {
        self.selected_capability_id = Some(capability_id);
        self
    }

    /// Loads the full on-chain audit trail object.
    ///
    /// Each call fetches a fresh snapshot from chain state rather than reusing cached client-side data.
    pub async fn get(&self) -> Result<OnChainAuditTrail, Error>
    where
        C: AuditTrailReadOnly,
    {
        trail_reader::get_audit_trail(self.trail_id, self.client).await
    }

    /// Updates the trail's mutable metadata field.
    ///
    /// Passing `None` clears the field on-chain.
    pub fn update_metadata<S>(&self, metadata: Option<String>) -> TransactionBuilder<UpdateMetadata>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(UpdateMetadata::new(
            self.trail_id,
            owner,
            metadata,
            self.selected_capability_id,
        ))
    }

    /// Migrates the trail to the latest package version supported by this crate.
    pub fn migrate<S>(&self) -> TransactionBuilder<Migrate>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(Migrate::new(self.trail_id, owner, self.selected_capability_id))
    }

    /// Deletes the trail object.
    ///
    /// Requires the `DeleteAuditTrail` permission. Deletion additionally requires the trail to be
    /// empty (`ETrailNotEmpty` otherwise) and the configured `delete_trail_lock` to have elapsed
    /// (`ETrailDeleteLocked` otherwise).
    pub fn delete_audit_trail<S>(&self) -> TransactionBuilder<DeleteAuditTrail>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(DeleteAuditTrail::new(self.trail_id, owner, self.selected_capability_id))
    }

    /// Returns the record API scoped to this trail.
    ///
    /// Use this for record reads, appends, and deletions.
    pub fn records(&self) -> TrailRecords<'a, C, Data> {
        TrailRecords::new(self.client, self.trail_id, self.selected_capability_id)
    }

    /// Returns the locking API scoped to this trail.
    ///
    /// Use this for inspecting lock state and updating locking rules.
    pub fn locking(&self) -> TrailLocking<'a, C> {
        TrailLocking::new(self.client, self.trail_id, self.selected_capability_id)
    }

    /// Returns the access-control API scoped to this trail.
    ///
    /// Use this for roles, capabilities, and access-policy updates.
    pub fn access(&self) -> TrailAccess<'a, C> {
        TrailAccess::new(self.client, self.trail_id, self.selected_capability_id)
    }

    /// Returns the tag-registry API scoped to this trail.
    ///
    /// Use this for managing the canonical tag registry that record writes and role tags must reference.
    pub fn tags(&self) -> TrailTags<'a, C> {
        TrailTags::new(self.client, self.trail_id, self.selected_capability_id)
    }
}
