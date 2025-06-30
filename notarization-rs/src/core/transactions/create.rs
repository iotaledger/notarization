// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Notarization
//!
//! This module defines the notarization struct and the operations for notarizations.
//!
//! ## Overview
//!
//! The notarization is a struct that contains the state, metadata, and operations for a notarization.

use async_trait::async_trait;
use iota_interaction::rpc_types::{
    IotaData as _, IotaObjectDataOptions, IotaTransactionBlockEffects, IotaTransactionBlockEvents,
};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaClientTrait, OptionalSend, OptionalSync};
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use serde::de::DeserializeOwned;
use tokio::sync::OnceCell;

use super::super::builder::NotarizationBuilder;
use super::super::operations::{NotarizationImpl, NotarizationOperations};
use super::super::types::{
    DynamicNotarizationCreated, Event, LockMetadata, LockedNotarizationCreated, NotarizationMethod,
    OnChainNotarization, TimeLock,
};
use crate::error::Error;
use crate::package::notarization_package_id;

/// A transaction that creates a new notarization.
#[derive(Debug, Clone)]
pub struct CreateNotarization<M> {
    builder: NotarizationBuilder<M>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl<M: Clone> CreateNotarization<M> {
    /// Creates a new [`CreateNotarization`] instance.
    pub fn new(builder: NotarizationBuilder<M>) -> Self {
        Self {
            builder,
            cached_ptb: OnceCell::new(),
        }
    }

    /// Indicates if the invariants for `NotarizationMethod::Dynamic` are satisfied:
    ///
    /// - Dynamic notarization can only have transfer locking or no `immutable_metadata.locking`. If
    ///   `immutable_metadata.locking` exists, all locks except `transfer_lock` must be `TimeLock::None` and the
    ///   `transfer_lock` must not be `TimeLock::None`.
    fn are_dynamic_notarization_invariants_ok(locking: &Option<LockMetadata>) -> bool {
        match locking {
            Some(lock_metadata) => {
                lock_metadata.delete_lock == TimeLock::None
                    && lock_metadata.update_lock == TimeLock::None
                    && lock_metadata.transfer_lock != TimeLock::None
            }
            None => true,
        }
    }

    /// Indicates if the invariants for `NotarizationMethod::Locked` are satisfied:
    ///
    /// - `locking` must exist.
    /// - `update_lock` and `transfer_lock` must be `TimeLock::UntilDestroyed`.
    fn are_locked_notarization_invariants_ok(locking: &Option<LockMetadata>) -> bool {
        match locking {
            Some(lock_metadata) => {
                lock_metadata.transfer_lock == TimeLock::UntilDestroyed
                    && lock_metadata.update_lock == TimeLock::UntilDestroyed
            }
            None => false,
        }
    }

    /// Makes a [`ProgrammableTransaction`] for the [`CreateNotarization`] instance.
    async fn make_ptb(&self, client: &impl CoreClientReadOnly) -> Result<ProgrammableTransaction, Error> {
        let NotarizationBuilder {
            state,
            immutable_description,
            updatable_metadata,
            method,
            delete_lock,
            transfer_lock,
            ..
        } = self.builder.clone();

        let package_id = notarization_package_id(client).await?;

        let state = state.ok_or_else(|| Error::InvalidArgument("State is required".to_string()))?;

        match method {
            NotarizationMethod::Dynamic => {
                if delete_lock.is_some() {
                    return Err(Error::InvalidArgument(
                        "Delete lock cannot be set for dynamic notarizations".to_string(),
                    ));
                }

                // Construct the locking metadata for dynamic notarization
                let locking = transfer_lock.as_ref().map(|t_lock| LockMetadata {
                    update_lock: TimeLock::None,
                    delete_lock: TimeLock::None,
                    transfer_lock: t_lock.clone(),
                });

                // Check invariants
                if !Self::are_dynamic_notarization_invariants_ok(&locking) {
                    return Err(Error::InvalidArgument(
                        "Dynamic notarization invariants are not satisfied".to_string(),
                    ));
                }

                NotarizationImpl::new_dynamic(
                    package_id,
                    state,
                    immutable_description,
                    updatable_metadata,
                    transfer_lock.unwrap_or(TimeLock::None),
                )
            }
            NotarizationMethod::Locked => {
                if transfer_lock.is_some() {
                    return Err(Error::InvalidArgument(
                        "Transfer lock cannot be set for locked notarizations".to_string(),
                    ));
                }

                // Construct the locking metadata for locked notarization
                let locking = Some(LockMetadata {
                    update_lock: TimeLock::UntilDestroyed,
                    delete_lock: delete_lock.clone().unwrap_or(TimeLock::None),
                    transfer_lock: TimeLock::UntilDestroyed,
                });

                // Check invariants
                if !Self::are_locked_notarization_invariants_ok(&locking) {
                    return Err(Error::InvalidArgument(
                        "Locked notarization invariants are not satisfied".to_string(),
                    ));
                }

                NotarizationImpl::new_locked(
                    package_id,
                    state,
                    immutable_description,
                    updatable_metadata,
                    delete_lock.unwrap_or(TimeLock::None),
                )
            }
        }
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl<M: Clone + OptionalSend + OptionalSync> Transaction for CreateNotarization<M> {
    type Error = Error;

    type Output = OnChainNotarization;

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
        client: &C,
    ) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let method = self.builder.method.clone();
        let data = events
            .data
            .first()
            .ok_or_else(|| Error::TransactionUnexpectedResponse("events should be provided".to_string()))?;

        let notarization_id = match method {
            NotarizationMethod::Dynamic => {
                let event: Event<DynamicNotarizationCreated> = serde_json::from_value(data.parsed_json.clone())
                    .map_err(|e| Error::TransactionUnexpectedResponse(format!("failed to parse event: {e}")))?;

                event.data.notarization_id
            }
            NotarizationMethod::Locked => {
                let event: Event<LockedNotarizationCreated> = serde_json::from_value(data.parsed_json.clone())
                    .map_err(|e| Error::TransactionUnexpectedResponse(format!("failed to parse event: {e}")))?;

                event.data.notarization_id
            }
        };

        let notarization = get_object_ref_by_id_with_bcs::<OnChainNotarization>(client, &notarization_id)
            .await
            .map_err(|e| Error::ObjectLookup(e.to_string()))?;

        Ok(notarization)
    }

    async fn apply<C>(mut self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        unreachable!()
    }
}

pub(crate) async fn get_object_ref_by_id_with_bcs<T: DeserializeOwned>(
    client: &impl CoreClientReadOnly,
    object_id: &ObjectID,
) -> Result<T, Error> {
    let notarization = client
        .client_adapter()
        .read_api()
        .get_object_with_options(*object_id, IotaObjectDataOptions::bcs_lossless())
        .await
        .map_err(|err| Error::ObjectLookup(err.to_string()))?
        .data
        .ok_or_else(|| Error::ObjectLookup("missing data in response".to_string()))?
        .bcs
        .ok_or_else(|| Error::ObjectLookup("missing object content in data".to_string()))?
        .try_into_move()
        .ok_or_else(|| Error::ObjectLookup("failed to convert data to move object".to_string()))?
        .deserialize()
        .map_err(|err| Error::ObjectLookup(err.to_string()))?;

    Ok(notarization)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_notarization_invariants() {
        let are_dynamic_notarization_invariants_ok = CreateNotarization::<()>::are_dynamic_notarization_invariants_ok;

        assert!(are_dynamic_notarization_invariants_ok(&None));
        assert!(are_dynamic_notarization_invariants_ok(&Some(LockMetadata {
            update_lock: TimeLock::None,
            delete_lock: TimeLock::None,
            transfer_lock: TimeLock::UntilDestroyed,
        })));
        assert!(!are_dynamic_notarization_invariants_ok(&Some(LockMetadata {
            update_lock: TimeLock::None,
            delete_lock: TimeLock::None,
            transfer_lock: TimeLock::None,
        })));
    }

    #[test]
    fn test_locked_notarization_invariants() {
        let are_locked_notarization_invariants_ok = CreateNotarization::<()>::are_locked_notarization_invariants_ok;

        assert!(!are_locked_notarization_invariants_ok(&None));
        assert!(are_locked_notarization_invariants_ok(&Some(LockMetadata {
            update_lock: TimeLock::UntilDestroyed,
            delete_lock: TimeLock::UntilDestroyed,
            transfer_lock: TimeLock::UntilDestroyed,
        })));
        assert!(!are_locked_notarization_invariants_ok(&Some(LockMetadata {
            update_lock: TimeLock::UntilDestroyed,
            delete_lock: TimeLock::UntilDestroyed,
            transfer_lock: TimeLock::None,
        })));
    }
}
