// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::{IotaTransactionBlockEffects, IotaTransactionBlockEvents};
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::operations::{CreateOps, CreateTrailArgs};
use crate::core::builder::AuditTrailBuilder;
use crate::core::internal::trail as trail_reader;
use crate::core::types::{AuditTrailCreated, Event, OnChainAuditTrail};
use crate::error::Error;

/// Output of a successful trail-creation transaction.
#[derive(Debug, Clone)]
pub struct TrailCreated {
    /// Newly created trail object ID.
    pub trail_id: ObjectID,
    /// Address that created the trail.
    pub creator: IotaAddress,
    /// Millisecond timestamp emitted by the creation event.
    pub timestamp: u64,
}

impl TrailCreated {
    /// Loads the newly created trail object from the ledger.
    ///
    /// # Errors
    ///
    /// Returns an error if the trail cannot be fetched or deserialized.
    pub async fn fetch_audit_trail<C>(&self, client: &C) -> Result<OnChainAuditTrail, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        trail_reader::get_audit_trail(self.trail_id, client).await
    }
}

/// A transaction that creates a new audit trail.
///
/// The builder state is normalized into the exact Move `create` call shape, including tag-registry setup,
/// optional initial-record creation, and initial-admin capability assignment.
///
/// On execution the Move package: shares the trail object, seeds the reserved `Admin` role with the
/// permissions returned by `permission::admin_permissions`, transfers a freshly minted initial-admin
/// capability to the admin address, stores the optional initial record at sequence number `0`, and emits
/// an `AuditTrailCreated` event. If an initial record carries a tag, the tag must already be in the
/// configured record-tag registry or the call aborts with `ERecordTagNotDefined`.
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
            initial_record,
            locking_config,
            trail_metadata,
            updatable_metadata,
            record_tags,
        } = self.builder.clone();

        let admin = admin.ok_or_else(|| {
            Error::InvalidArgument(
                "admin address is required; use `client.create_trail()` with signer or call `with_admin(...)`"
                    .to_string(),
            )
        })?;
        let tf_package_id = client
            .tf_components_package_id()
            .expect("TfComponents package ID should be present for audit trail clients");

        CreateOps::create_trail(CreateTrailArgs {
            audit_trail_package_id: client.package_id(),
            tf_components_package_id: tf_package_id,
            admin,
            initial_record,
            locking_config,
            trail_metadata,
            updatable_metadata,
            record_tags,
        })
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
