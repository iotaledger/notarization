// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::OptionalSync;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_number_from_string;
use tokio::sync::OnceCell;

use super::operations::{NotarizationImpl, NotarizationOperations};
use super::timelock::LockMetadata;
use crate::error::Error;
use crate::package::notarization_package_id;

/// The immutable metadata of a notarization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImmutableMetadata {
    /// Timestamp when the `Notarization` was created
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub created_at: u64,
    /// Description of the `Notarization`
    pub description: Option<String>,
    /// Optional lock metadata for `Notarization`
    pub locking: Option<LockMetadata>,
}

pub struct UpdateMetadata {
    metadata: Option<String>,
    object_id: ObjectID,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateMetadata {
    pub fn new(metadata: Option<String>, object_id: ObjectID) -> Self {
        Self {
            metadata,
            object_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let package_id = notarization_package_id(client).await?;

        NotarizationImpl::update_metadata(client, package_id, self.object_id, self.metadata.clone()).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for UpdateMetadata {
    type Error = Error;

    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}
