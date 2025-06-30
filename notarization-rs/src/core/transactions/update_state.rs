// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Update State
//!
//! This module defines the update state transaction.
//!
//! ## Overview
//!
//! The update state transaction is used to update the state of a notarization.
//!
//! Note that this transaction is only available for dynamic notarizations.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::super::operations::{NotarizationImpl, NotarizationOperations};
use super::super::types::State;
use crate::error::Error;

/// A transaction that updates the state of an existing notarization.
///
/// This transaction can only be used with dynamic notarizations, as locked
/// notarizations are immutable after creation.
///
/// ## Example
///
/// ```rust,no_run
/// # use notarization::core::transactions::UpdateState;
/// # use notarization::core::types::State;
/// # use iota_interaction::types::base_types::ObjectID;
/// # use std::str::FromStr;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let new_state = State::from_string(
///     "Updated content v2".to_string(),
///     Some("Second revision".to_string()),
/// );
///
/// let object_id = ObjectID::from_str("0x123...")?;
/// let update_tx = UpdateState::new(new_state, object_id);
/// # Ok(())
/// # }
/// ```
pub struct UpdateState {
    state: State,
    object_id: ObjectID,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateState {
    /// Creates a new state update transaction.
    ///
    /// ## Parameters
    ///
    /// - `state`: The new state to set
    /// - `object_id`: The ID of the notarization to update
    pub fn new(state: State, object_id: ObjectID) -> Self {
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
