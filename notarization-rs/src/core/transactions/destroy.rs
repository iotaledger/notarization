// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Destroy Notarization
//!
//! This module defines the destroy-notarization transaction.
//!
//! ## Overview
//!
//! The destroy-notarization transaction permanently destroys a notarization
//! and releases its object ID. All package-local [`TimeLock`](super::super::types::TimeLock)s
//! of the attached [`LockMetadata`](super::super::types::LockMetadata) are
//! destroyed in the process. The on-chain gating check `is_destroy_allowed`
//! ensures that no `UnlockAt` lock is still active.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_sdk_types::{ObjectId, ProgrammableTransaction};
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::super::operations::{NotarizationImpl, NotarizationOperations};
use crate::error::Error;

/// A transaction that destroys a notarization on-chain and releases its
/// object ID.
///
/// All package-local [`TimeLock`](super::super::types::TimeLock)s of the
/// attached [`LockMetadata`](super::super::types::LockMetadata) are
/// destroyed in the process. The notarization must currently be
/// destroy-allowed (see
/// [`NotarizationClientReadOnly::is_destroy_allowed`](crate::client::NotarizationClientReadOnly::is_destroy_allowed));
/// otherwise the on-chain transaction aborts.
///
/// Emits a `NotarizationDestroyed` event on success.
pub struct DestroyNotarization {
    notarization_id: ObjectId,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl DestroyNotarization {
    /// Creates a new destroy transaction.
    pub fn new(notarization_id: ObjectId) -> Self {
        Self {
            notarization_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::destroy(client, self.notarization_id).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for DestroyNotarization {
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
