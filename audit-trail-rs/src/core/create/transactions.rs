// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_interaction::rpc_types::{
    IotaData as _, IotaObjectDataOptions, IotaTransactionBlockEffects, IotaTransactionBlockEvents,
};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaClientTrait, OptionalSync};
use iota_sdk::types::base_types::IotaAddress;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::operations::CreateOps;
use crate::core::builder::AuditTrailBuilder;
use crate::core::operations;
use crate::core::types::{AuditTrailCreated, Event, OnChainAuditTrail};
use crate::error::Error;

/// Output of a create trail transaction.
#[derive(Debug, Clone)]
pub struct TrailCreated {
    pub trail_id: ObjectID,
    pub creator: IotaAddress,
    pub timestamp: u64,
}

impl TrailCreated {
    pub async fn fetch_audit_trail<C>(&self, client: &C) -> Result<OnChainAuditTrail, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::get_audit_trail(self.trail_id, client).await
    }
}

/// A transaction that creates a new audit trail.
#[derive(Debug, Clone)]
pub struct CreateTrail {
    builder: AuditTrailBuilder,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl CreateTrail {
    /// Creates a new [`CreateTrail`] instance.
    pub fn new(builder: AuditTrailBuilder) -> Self {
        Self {
            builder,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let AuditTrailBuilder {
            admin,
            record: data,
            record_metadata,
            locking_config,
            trail_metadata,
            updatable_metadata,
        } = self.builder.clone();

        let admin = admin.ok_or_else(|| {
            Error::InvalidArgument(
                "admin address is required; use `client.create_trail()` with signer or call `with_admin(...)`"
                    .to_string(),
            )
        })?;

        CreateOps::create_trail(
            client.package_id(),
            admin,
            data,
            record_metadata,
            locking_config,
            trail_metadata,
            updatable_metadata,
        )
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for CreateTrail {
    type Error = Error;
    type Output = TrailCreated;

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
            .find_map(|data| serde_json::from_value::<Event<AuditTrailCreated>>(data.parsed_json.clone()).ok())
            .ok_or_else(|| Error::UnexpectedApiResponse("AuditTrailCreated event not found".to_string()))?;

        Ok(TrailCreated {
            trail_id: event.data.trail_id,
            creator: event.data.creator,
            timestamp: event.data.timestamp,
        })
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}
