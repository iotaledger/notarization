// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Update State
//!
//! This module defines the update-state transaction.
//!
//! ## Overview
//!
//! The update-state transaction replaces the `state` of an existing
//! notarization. Behaviour depends on the Notarization Method:
//! * `Dynamic`: always permitted — the underlying `update_lock` is fixed to `TimeLock::None`.
//! * `Locked`: always aborts on-chain, because the underlying `update_lock` is pinned to `TimeLock::UntilDestroyed`.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_sdk_types::{ObjectId, ProgrammableTransaction};
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::super::operations::{NotarizationImpl, NotarizationOperations};
use super::super::types::State;
use crate::error::Error;

/// A transaction that replaces the `state` of an existing notarization.
///
/// On success the on-chain transaction bumps
/// `OnChainNotarization::state_version_count` by one and refreshes
/// `OnChainNotarization::last_state_change_at` to the on-chain clock
/// timestamp (in milliseconds since the Unix epoch).
///
/// Behaviour depends on the Notarization Method:
/// * `Dynamic`: always permitted — the underlying `update_lock` is fixed to `TimeLock::None`.
/// * `Locked`: always aborts on-chain, because the underlying `update_lock` is pinned to `TimeLock::UntilDestroyed`.
///
/// Emits a `NotarizationUpdated` event on success.
///
/// ## Example
///
/// ```rust,no_run
/// # use notarization::core::transactions::UpdateState;
/// # use notarization::core::types::State;
/// # use iota_sdk_types::ObjectId;
/// # use std::str::FromStr;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let new_state = State::from_string(
///     "Updated content v2".to_string(),
///     Some("Second revision".to_string()),
/// );
///
/// let object_id = ObjectId::from_str("0x123...")?;
/// let update_tx = UpdateState::new(new_state, object_id);
/// # Ok(())
/// # }
/// ```
pub struct UpdateState {
    state: State,
    object_id: ObjectId,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateState {
    /// Creates a new state update transaction.
    ///
    /// ## Parameters
    ///
    /// - `state`: The new state to set
    /// - `object_id`: The ID of the notarization to update
    pub fn new(state: State, object_id: ObjectId) -> Self {
        Self {
            state,
            object_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let new_state = self.state.clone();

        NotarizationImpl::update_state(client, self.object_id, new_state).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for UpdateState {
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
