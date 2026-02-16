// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::{IotaKeySignature, OptionalSync};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
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

pub use transactions::UpdateMetadata;

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
        self.client.get_object_by_id(self.trail_id).await.map_err(|err| {
            Error::UnexpectedApiResponse(format!(
                "failed to load on-chain trail {}; {err}",
                self.trail_id
            ))
        })
    }

    /// Updates the trail's updatable metadata.
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
