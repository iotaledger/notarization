// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Update Metadata
//!
//! This module defines the update-metadata transaction.
//!
//! ## Overview
//!
//! The update-metadata transaction replaces the `updatable_metadata` of an
//! existing notarization. It does not affect `state`, `state_version_count`,
//! `last_state_change_at`, or the immutable description. Behaviour depends
//! on the Notarization Method:
//! * `Dynamic`: always permitted — the underlying `update_lock` is fixed to `TimeLock::None`.
//! * `Locked`: always aborts on-chain, because the underlying `update_lock` is pinned to `TimeLock::UntilDestroyed`.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_sdk_types::ObjectId;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::super::operations::{NotarizationImpl, NotarizationOperations};
use crate::error::Error;

/// A transaction that replaces the `updatable_metadata` of an existing
/// notarization.
///
/// Does not affect `state`, `state_version_count`, `last_state_change_at`,
/// or the immutable description.
///
/// Behaviour depends on the Notarization Method:
/// * `Dynamic`: always permitted — the underlying `update_lock` is fixed to `TimeLock::None`.
/// * `Locked`: always aborts on-chain, because the underlying `update_lock` is pinned to `TimeLock::UntilDestroyed`.
pub struct UpdateMetadata {
    metadata: Option<String>,
    /// The ID of the notarization to update
    notarization_id: ObjectId,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateMetadata {
    /// Creates a new transaction for updating the metadata of a notarization.
    pub fn new(metadata: Option<String>, notarization_id: ObjectId) -> Self {
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
