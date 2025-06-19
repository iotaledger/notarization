// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Notarization Metadata
//!
//! This module defines the metadata for notarizations.
//!
//! ## Overview
//!
//! The metadata is used to store the metadata for a notarization.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::super::operations::{NotarizationImpl, NotarizationOperations};
use crate::error::Error;

/// A transaction that updates the metadata of a notarization.
pub struct UpdateMetadata {
    metadata: Option<String>,
    /// The ID of the notarization to update
    notarization_id: ObjectID,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateMetadata {
    /// Creates a new transaction for updating the metadata of a notarization.
    pub fn new(metadata: Option<String>, notarization_id: ObjectID) -> Self {
        Self {
            metadata,
            notarization_id,
            cached_ptb: OnceCell::new(),
        }
    }

    /// Builds the programmable transaction for updating the metadata of a
    /// notarization.
    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::update_metadata(client, self.notarization_id, self.metadata.clone()).await
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
