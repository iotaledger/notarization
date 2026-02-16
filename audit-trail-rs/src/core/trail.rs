// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::rpc_types::{
    IotaData as _, IotaObjectDataOptions, IotaTransactionBlockEffects, IotaTransactionBlockEvents,
};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaClientTrait, IotaKeySignature, OptionalSync};
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
        let data = self
            .client
            .client_adapter()
            .read_api()
            .get_object_with_options(self.trail_id, IotaObjectDataOptions::bcs_lossless())
            .await
            .map_err(|e| Error::UnexpectedApiResponse(format!("failed to fetch trail {} object; {e}", self.trail_id)))?
            .data
            .ok_or_else(|| Error::UnexpectedApiResponse(format!("trail {} data not found", self.trail_id)))?;

        data.bcs
            .ok_or_else(|| Error::UnexpectedApiResponse(format!("trail {} missing bcs object content", self.trail_id)))?
            .try_into_move()
            .ok_or_else(|| {
                Error::UnexpectedApiResponse(format!("trail {} bcs content is not a move object", self.trail_id))
            })?
            .deserialize()
            .map_err(|e| {
                Error::UnexpectedApiResponse(format!("failed to decode trail {} bcs data; {e}", self.trail_id))
            })
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
