// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::{IotaTransactionBlockEffects, IotaTransactionBlockEvents};
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use crate::core::operations::AuditTrailImpl;
use crate::core::types::{Data, Event, RecordAdded, RecordDeleted};
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct AddRecord {
    pub trail_id: ObjectID,
    pub owner: IotaAddress,
    pub data: Data,
    pub metadata: Option<String>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl AddRecord {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, data: Data, metadata: Option<String>) -> Self {
        Self {
            trail_id,
            owner,
            data,
            metadata,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AuditTrailImpl::add_record(
            client,
            self.trail_id,
            self.owner,
            self.data.clone(),
            self.metadata.clone(),
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
        for data in &events.data {
            if let Ok(event) = serde_json::from_value::<Event<RecordAdded>>(data.parsed_json.clone()) {
                return Ok(event.data);
            }
        }

        Err(Error::UnexpectedApiResponse("RecordAdded event not found".to_string()))
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

#[derive(Debug, Clone)]
pub struct DeleteRecord {
    pub trail_id: ObjectID,
    pub owner: IotaAddress,
    pub sequence_number: u64,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DeleteRecord {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, sequence_number: u64) -> Self {
        Self {
            trail_id,
            owner,
            sequence_number,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        AuditTrailImpl::delete_record(client, self.trail_id, self.owner, self.sequence_number).await
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
        for data in &events.data {
            if let Ok(event) = serde_json::from_value::<Event<RecordDeleted>>(data.parsed_json.clone()) {
                return Ok(event.data);
            }
        }

        Err(Error::UnexpectedApiResponse(
            "RecordDeleted event not found".to_string(),
        ))
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}
