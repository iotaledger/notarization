// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Notarization Operations
//!
//! This module defines the operations for notarizations.
//!
//! ## Overview
//!
//! The operations are used to build transactions for notarizations.

use std::str::FromStr;

use async_trait::async_trait;
use iota_interaction::types::Identifier;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::{Argument, ObjectArg, ProgrammableTransaction};
use iota_interaction::{OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;

use super::move_utils;
use super::types::{State, TimeLock};
use crate::error::Error;

/// Internal implementation of notarization operations.
#[derive(Debug, Clone)]
pub(crate) struct NotarizationImpl;

impl NotarizationImpl {
    /// Helper to create a new builder and run a closure that injects the
    /// creation logic.
    ///
    /// # Arguments
    /// * `iota_client` - The IOTA client adapter
    /// * `package_id` - The package ID for the transaction
    /// * `object_id` - Optional object ID for the notarization
    /// * `method` - The method name to call
    /// * `additional_args` - Closure providing additional arguments for the transaction
    ///
    /// # Type Parameters
    /// * `F` - Closure type that produces additional arguments
    ///
    /// # Errors
    /// Returns `Error` if:
    /// * Tag retrieval fails
    /// * Object reference retrieval fails
    /// * Transaction building fails
    /// * Method name is invalid
    async fn build_transaction<C, F>(
        client: &C,
        object_id: ObjectID,
        method: impl AsRef<str>,
        additional_args: F,
    ) -> Result<ProgrammableTransaction, Error>
    where
        F: FnOnce(&mut ProgrammableTransactionBuilder) -> Result<Vec<Argument>, Error>,
        C: CoreClientReadOnly + OptionalSync,
    {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let tag = vec![move_utils::get_type_tag(client, &object_id).await?];

        let mut args = {
            let notarization = move_utils::get_object_ref_by_id(client, &object_id).await?;

            vec![
                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(format!("Failed to create object argument: {e}")))?,
            ]
        };
        // Add additional arguments
        args.extend(
            additional_args(&mut ptb)
                .map_err(|e| Error::InvalidArgument(format!("Failed to add additional arguments: {e}")))?,
        );

        // Create method identifier
        let function = Identifier::from_str(method.as_ref())
            .map_err(|e| Error::InvalidArgument(format!("Invalid method name '{}': {}", method.as_ref(), e)))?;

        // Build the move call
        ptb.programmable_move_call(
            client.package_id(),
            ident_str!("notarization").into(),
            function,
            tag,
            args,
        );

        Ok(ptb.finish())
    }
}

/// Notarization operations
///
/// These operations return a `ProgrammableTransaction` which is
/// a single transaction, or command, in a programmable transaction block
#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
pub(crate) trait NotarizationOperations {
    /// Build a transaction that creates a new locked notarization
    fn new_locked(
        package_id: ObjectID,
        state: State,
        immutable_description: Option<String>,
        updatable_metadata: Option<String>,
        delete_lock: TimeLock,
    ) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let tag = state.data.tag();
        let clock = move_utils::get_clock_ref(&mut ptb);
        let state_arg = state.into_ptb(&mut ptb, package_id)?;
        let immutable_description = move_utils::ptb_pure(&mut ptb, "immutable_description", immutable_description)?;
        let updatable_metadata = move_utils::ptb_pure(&mut ptb, "updatable_metadata", updatable_metadata)?;
        let delete_lock = delete_lock.to_ptb(&mut ptb, package_id)?;

        ptb.programmable_move_call(
            package_id,
            ident_str!("locked_notarization").into(),
            ident_str!("create").into(),
            vec![tag],
            vec![state_arg, immutable_description, updatable_metadata, delete_lock, clock],
        );

        Ok(ptb.finish())
    }

    /// Build a transaction that creates a new dynamic notarization
    fn new_dynamic(
        package_id: ObjectID,
        state: State,
        immutable_description: Option<String>,
        updatable_metadata: Option<String>,
        transfer_lock: TimeLock,
    ) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let tag = state.data.tag();
        let clock = move_utils::get_clock_ref(&mut ptb);
        let state_arg = state.into_ptb(&mut ptb, package_id)?;
        let immutable_description = move_utils::ptb_pure(&mut ptb, "immutable_description", immutable_description)?;
        let updatable_metadata = move_utils::ptb_pure(&mut ptb, "updatable_metadata", updatable_metadata)?;
        let transfer_lock = transfer_lock.to_ptb(&mut ptb, package_id)?;

        ptb.programmable_move_call(
            package_id,
            ident_str!("dynamic_notarization").into(),
            ident_str!("create").into(),
            vec![tag],
            vec![
                state_arg,
                immutable_description,
                updatable_metadata,
                transfer_lock,
                clock,
            ],
        );

        Ok(ptb.finish())
    }

    /// Build a transaction that updates the state of a notarization
    async fn update_state<C>(
        client: &C,
        object_id: ObjectID,
        new_state: State,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "update_state", |ptb| {
            Ok(vec![
                new_state.into_ptb(ptb, client.package_id())?,
                move_utils::get_clock_ref(ptb),
            ])
        })
        .await
    }

    /// Build a transaction that destroys a notarization
    async fn destroy<C>(client: &C, object_id: ObjectID) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "destroy", |ptb| {
            Ok(vec![move_utils::get_clock_ref(ptb)])
        })
        .await
    }

    /// Build a transaction that updates the metadata of a notarization
    async fn update_metadata<C>(
        client: &C,
        object_id: ObjectID,
        new_metadata: Option<String>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "update_metadata", |ptb| {
            Ok(vec![
                move_utils::ptb_pure(ptb, "new_metadata", new_metadata)?,
                move_utils::get_clock_ref(ptb),
            ])
        })
        .await
    }

    /// Build a transaction that returns the notarization method
    async fn notarization_method<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "notarization_method", |_| Ok(vec![])).await
    }

    /// Build a transaction that checks if the notarization is locked for update
    async fn is_update_locked<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "is_update_locked", |ptb| {
            Ok(vec![move_utils::get_clock_ref(ptb)])
        })
        .await
    }

    /// Build a transaction that checks if the notarization is allowed to be destroyed
    async fn is_destroy_allowed<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "is_destroy_allowed", |ptb| {
            Ok(vec![move_utils::get_clock_ref(ptb)])
        })
        .await
    }

    /// Build a transaction that checks if the notarization is locked for transfer
    async fn is_transfer_locked<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "is_transfer_locked", |ptb| {
            Ok(vec![move_utils::get_clock_ref(ptb)])
        })
        .await
    }

    /// Last change timestamp
    async fn last_change_ts<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "last_change", |_| Ok(vec![])).await
    }

    /// Version count
    async fn version_count<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "version_count", |_| Ok(vec![])).await
    }

    /// Created at timestamp
    async fn created_at<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "created_at", |_| Ok(vec![])).await
    }

    /// Description
    async fn description<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "description", |_| Ok(vec![])).await
    }

    /// Updatable metadata
    async fn updatable_metadata<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "updatable_metadata", |_| Ok(vec![])).await
    }

    /// Lock metadata
    async fn lock_metadata<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "lock_metadata", |_| Ok(vec![])).await
    }

    async fn state<C>(object_id: ObjectID, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        NotarizationImpl::build_transaction(client, object_id, "state", |_| Ok(vec![])).await
    }

    async fn transfer_notarization<C>(
        object_id: ObjectID,
        recipient: IotaAddress,
        client: &C,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let mut ptb = ProgrammableTransactionBuilder::new();
        let tag = vec![move_utils::get_type_tag(client, &object_id).await?];
        let recipient = ptb
            .pure(recipient)
            .map_err(|e| Error::InvalidArgument(format!("Failed to create recipient argument: {e}")))?;

        let notarization = move_utils::get_object_ref_by_id(client, &object_id).await?;
        let notarization = ptb
            .obj(ObjectArg::ImmOrOwnedObject(notarization))
            .map_err(|e| Error::InvalidArgument(format!("Failed to create notarization argument: {e}")))?;

        let clock = move_utils::get_clock_ref(&mut ptb);

        ptb.programmable_move_call(
            client.package_id(),
            ident_str!("dynamic_notarization").into(),
            ident_str!("transfer").into(),
            tag,
            vec![notarization, recipient, clock],
        );

        Ok(ptb.finish())
    }
}

impl NotarizationOperations for NotarizationImpl {}
