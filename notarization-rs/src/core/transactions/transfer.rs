// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Transfer Notarization
//!
//! This module defines the transfer-notarization transaction.
//!
//! ## Overview
//!
//! The transfer-notarization transaction transfers ownership of a
//! Dynamic-Notarization to a new address. Permitted only when the
//! notarization has no [`LockMetadata`](super::super::types::LockMetadata)
//! or when its `transfer_lock` is not currently active.
//!
//! Behaviour depends on the Notarization Method:
//! * `Dynamic`: gated by the configured `transfer_lock`. Submitting while the lock is engaged aborts on-chain.
//! * `Locked`: always aborts on-chain — Locked-Notarizations have their `transfer_lock` pinned to
//!   `TimeLock::UntilDestroyed` and are therefore non-transferable.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_sdk_types::{ObjectId, ProgrammableTransaction, Address};
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::super::operations::{NotarizationImpl, NotarizationOperations};
use crate::error::Error;

/// A transaction that transfers ownership of a Dynamic-Notarization to
/// another address.
///
/// Permitted only when the notarization has no
/// [`LockMetadata`](super::super::types::LockMetadata) or when its
/// `transfer_lock` is not currently active.
///
/// Behaviour depends on the Notarization Method:
/// * `Dynamic`: on success the notarization is transferred to `recipient`. Submitting while the configured
///   `transfer_lock` is engaged aborts on-chain.
/// * `Locked`: always aborts on-chain — Locked-Notarizations have their `transfer_lock` pinned to
///   `TimeLock::UntilDestroyed` and are therefore non-transferable.
///
/// Emits a `DynamicNotarizationTransferred` event on success.
pub struct TransferNotarization {
    recipient: Address,
    notarization_id: ObjectId,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl TransferNotarization {
    /// Creates a new transfer transaction.
    pub fn new(recipient: Address, notarization_id: ObjectId) -> Self {
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
