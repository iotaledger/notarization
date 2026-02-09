// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::ObjectID;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::CoreClient;
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;
use serde::de::DeserializeOwned;

use super::{AuditTrailFull, AuditTrailReadOnly};
use crate::core::operations::AuditTrailImpl;
use crate::core::transactions::{AddRecord, DeleteRecord};
use crate::core::types::{Data, Record};
use crate::error::Error;

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
        let tx = AuditTrailImpl::get_record(self.client, self.trail_id, sequence_number).await?;
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
        let tx = AuditTrailImpl::has_record(self.client, self.trail_id, sequence_number).await?;
        self.client.execute_read_only_transaction(tx).await
    }

    async fn first_sequence(&self) -> Result<Option<u64>, Error>
    where
        C: AuditTrailReadOnly,
    {
        let tx = AuditTrailImpl::first_sequence(self.client, self.trail_id).await?;
        self.client.execute_read_only_transaction(tx).await
    }

    async fn last_sequence(&self) -> Result<Option<u64>, Error>
    where
        C: AuditTrailReadOnly,
    {
        let tx = AuditTrailImpl::last_sequence(self.client, self.trail_id).await?;
        self.client.execute_read_only_transaction(tx).await
    }

    pub async fn record_count(&self) -> Result<u64, Error>
    where
        C: AuditTrailReadOnly,
    {
        let tx = AuditTrailImpl::record_count(self.client, self.trail_id).await?;
        self.client.execute_read_only_transaction(tx).await
    }
}
