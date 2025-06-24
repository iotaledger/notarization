// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Transfer Notarization
//!
//! This module defines the transfer notarization transaction.
//!
//! ## Overview
//!
//! The transfer notarization transaction is used to transfer ownership of a dynamic notarization to a new address.
//!
//! Note that this transaction is only available for dynamic notarizations.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::super::operations::{NotarizationImpl, NotarizationOperations};
use crate::error::Error;

/// A transaction that transfers ownership of a dynamic notarization.
pub struct TransferNotarization {
    recipient: IotaAddress,
    notarization_id: ObjectID,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl TransferNotarization {
    /// Creates a new transfer transaction.
    pub fn new(recipient: IotaAddress, notarization_id: ObjectID) -> Self {
        Self {
            recipient,
            notarization_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::transfer_notarization(self.notarization_id, self.recipient, client).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for TransferNotarization {
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
