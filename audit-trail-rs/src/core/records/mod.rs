// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_interaction::rpc_types::{IotaTransactionBlockEffects, IotaTransactionBlockEvents};
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::transaction::transaction_builder::{Transaction, TransactionBuilder};
use secret_storage::Signer;
use serde::de::DeserializeOwned;
use tokio::sync::OnceCell;

use crate::core::trail::{AuditTrailFull, AuditTrailReadOnly};
use crate::core::types::{Data, Event, Record, RecordAdded, RecordDeleted};
use crate::error::Error;

mod operations;
use self::operations::RecordsOps;

#[derive(Debug, Clone)]
pub struct TrailRecords<'a, C, D = Data> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
    pub(crate) _phantom: std::marker::PhantomData<D>,
}

impl<'a, C, D> TrailRecords<'a, C, D> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self {
            client,
            trail_id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn get(&self, sequence_number: u64) -> Result<Record<D>, Error>
    where
        C: AuditTrailReadOnly,
        D: DeserializeOwned,
    {
        let tx = RecordsOps::get_record_tx(self.client, self.trail_id, sequence_number).await?;
        self.client.execute_read_only_transaction(tx).await
    }

    pub async fn list(&self) -> Result<Vec<Record<D>>, Error>
    where
        C: AuditTrailReadOnly,
        D: DeserializeOwned,
    {
        let first = self.first_sequence().await?;
        let last = self.last_sequence().await?;

        let Some(first_seq) = first else {
            return Ok(Vec::new());
        };
        let Some(last_seq) = last else {
            return Ok(Vec::new());
        };

        let mut records = Vec::new();
        for seq in first_seq..=last_seq {
            if self.has_record(seq).await? {
                records.push(self.get(seq).await?);
            }
        }

        Ok(records)
    }

    pub fn add<S>(&self, data: D, metadata: Option<String>) -> Result<TransactionBuilder<AddRecord>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
        D: Into<Data>,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(AddRecord::new(
            self.trail_id,
            owner,
            data.into(),
            metadata,
        )))
    }

    pub fn delete<S>(&self, sequence_number: u64) -> Result<TransactionBuilder<DeleteRecord>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(DeleteRecord::new(
            self.trail_id,
            owner,
            sequence_number,
        )))
    }

    pub async fn correct(&self, _replaces: Vec<u64>, _data: D, _metadata: Option<String>) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailRecords::correct"))
    }

    async fn has_record(&self, sequence_number: u64) -> Result<bool, Error>
    where
        C: AuditTrailReadOnly,
    {
        let tx = RecordsOps::has_record_tx(self.client, self.trail_id, sequence_number).await?;
        self.client.execute_read_only_transaction(tx).await
    }

    async fn first_sequence(&self) -> Result<Option<u64>, Error>
    where
        C: AuditTrailReadOnly,
    {
        let tx = RecordsOps::first_sequence_tx(self.client, self.trail_id).await?;
        self.client.execute_read_only_transaction(tx).await
    }

    async fn last_sequence(&self) -> Result<Option<u64>, Error>
    where
        C: AuditTrailReadOnly,
    {
        let tx = RecordsOps::last_sequence_tx(self.client, self.trail_id).await?;
        self.client.execute_read_only_transaction(tx).await
    }

    pub async fn record_count(&self) -> Result<u64, Error>
    where
        C: AuditTrailReadOnly,
    {
        let tx = RecordsOps::record_count_tx(self.client, self.trail_id).await?;
        self.client.execute_read_only_transaction(tx).await
    }
}

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
        RecordsOps::add_record_tx(
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
        RecordsOps::delete_record_tx(client, self.trail_id, self.owner, self.sequence_number).await
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
