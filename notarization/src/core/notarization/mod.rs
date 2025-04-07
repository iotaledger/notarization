// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::core::state::State;
use crate::core::timelock::TimeLock;
use crate::error::Error;
use identity_iota_interaction::ident_str;
use iota_sdk::types::programmable_transaction_builder::ProgrammableTransactionBuilder;

use std::future::Future;

use identity_iota_core::iota_interaction_rust::IotaClientAdapter;
use iota_sdk::types::base_types::ObjectID;

use iota_sdk::types::transaction::{ObjectArg, ProgrammableTransaction};

use super::utils;

#[derive(Debug, Clone)]
/// A unified notarization type that can be either dynamic or locked
pub struct Notarization;

/// Notarization operations
///
/// These operations return the ProgrammableTransactionBuilder
pub trait NotarizationOperations {
    /// Build a transaction that creates a new locked notarization
    fn new_locked(
        &self,
        state: State,
        immutable_description: Option<String>,
        updateable_metadata: Option<String>,
        delete_lock: TimeLock,
        package_id: ObjectID,
    ) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let tag = state.data.tag();
        let clock = utils::get_clock_ref(&mut ptb);
        let state_arg = state.to_ptb(&mut ptb, package_id)?;
        let immutable_description = utils::new_move_option_string(immutable_description, &mut ptb)?;
        let updateable_metadata = utils::new_move_option_string(updateable_metadata, &mut ptb)?;
        let delete_lock = delete_lock.to_ptb(&mut ptb, package_id)?;

        ptb.programmable_move_call(
            package_id,
            ident_str!("locked_notarization").into(),
            ident_str!("create").into(),
            vec![tag],
            vec![
                state_arg,
                immutable_description,
                updateable_metadata,
                delete_lock,
                clock,
            ],
        );

        Ok(ptb.finish())
    }

    /// Build a transaction that creates a new dynamic notarization
    fn new_dynamic(
        &self,
        state: State,
        immutable_description: Option<String>,
        updateable_metadata: Option<String>,
        transfer_lock: Option<TimeLock>,
        package_id: ObjectID,
    ) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let tag = state.data.tag();
        let clock = utils::get_clock_ref(&mut ptb);
        let state_arg = state.to_ptb(&mut ptb, package_id)?;
        let immutable_description = utils::new_move_option_string(immutable_description, &mut ptb)?;
        let updateable_metadata = utils::new_move_option_string(updateable_metadata, &mut ptb)?;
        let transfer_lock = utils::option_to_move(transfer_lock, &mut ptb, package_id)?;

        ptb.programmable_move_call(
            package_id,
            ident_str!("dynamic_notarization").into(),
            ident_str!("create").into(),
            vec![tag],
            vec![
                state_arg,
                immutable_description,
                updateable_metadata,
                transfer_lock,
                clock,
            ],
        );

        Ok(ptb.finish())
    }

    /// Build a transaction that updates the state of a notarization
    fn update_state(
        &self,
        iota_client: &IotaClientAdapter,
        object_id: ObjectID,
        package_id: ObjectID,
        new_state: State,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let clock = utils::get_clock_ref(&mut ptb);
            let tag = new_state.data.tag();
            let state_arg = new_state.to_ptb(&mut ptb, package_id)?;

            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };
            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("update_state").into(),
                vec![tag],
                vec![notarization, state_arg, clock],
            );

            Ok(ptb.finish())
        }
    }

    /// Build a transaction that destroys a notarization
    fn destroy(
        &self,
        iota_client: &IotaClientAdapter,
        object_id: ObjectID,
        package_id: ObjectID,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let clock = utils::get_clock_ref(&mut ptb);

            let tag = utils::get_type_tag(iota_client, object_id).await?;

            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("destroy").into(),
                vec![tag],
                vec![notarization, clock],
            );

            Ok(ptb.finish())
        }
    }

    /// Build a transaction that updates the metadata of a notarization
    fn update_metadata(
        &self,
        iota_client: &IotaClientAdapter,
        object_id: ObjectID,
        package_id: ObjectID,
        new_metadata: Option<String>,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let clock = utils::get_clock_ref(&mut ptb);
            let tag = utils::get_type_tag(iota_client, object_id).await?;
            let metadata = utils::new_move_option_string(new_metadata, &mut ptb)?;

            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("update_metadata").into(),
                vec![tag],
                vec![notarization, metadata, clock],
            );

            Ok(ptb.finish())
        }
    }

    /// Build a transaction that returns the notarization method
    fn notarization_method(
        &self,
        package_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let tag = utils::get_type_tag(iota_client, package_id).await?;
            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, package_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("notarization_method").into(),
                vec![tag],
                vec![notarization],
            );

            Ok(ptb.finish())
        }
    }

    /// Build a transaction that checks if the notarization is locked for update
    fn is_update_locked(
        &self,
        package_id: ObjectID,
        object_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let tag = utils::get_type_tag(iota_client, package_id).await?;
            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("is_update_locked").into(),
                vec![tag],
                vec![notarization],
            );

            Ok(ptb.finish())
        }
    }

    /// Build a transaction that checks if the notarization is locked for deletion
    fn is_destroy_locked(
        &self,
        package_id: ObjectID,
        object_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let tag = utils::get_type_tag(iota_client, package_id).await?;
            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("is_destroy_locked").into(),
                vec![tag],
                vec![notarization],
            );

            Ok(ptb.finish())
        }
    }

    /// Build a transaction that checks if the notarization is locked for transfer
    fn is_transfer_locked(
        &self,
        package_id: ObjectID,
        object_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let tag = utils::get_type_tag(iota_client, package_id).await?;
            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("is_transfer_locked").into(),
                vec![tag],
                vec![notarization],
            );

            Ok(ptb.finish())
        }
    }

    /// Build a transaction that checks if the notarization can be destroyed
    fn is_destroy_allowed(
        &self,
        package_id: ObjectID,
        object_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let tag = utils::get_type_tag(iota_client, package_id).await?;
            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("is_destroy_allowed").into(),
                vec![tag],
                vec![notarization],
            );

            Ok(ptb.finish())
        }
    }

    /// Last change timestamp
    fn last_change(
        &self,
        package_id: ObjectID,
        object_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let tag = utils::get_type_tag(iota_client, package_id).await?;
            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("last_change").into(),
                vec![tag],
                vec![notarization],
            );

            Ok(ptb.finish())
        }
    }

    /// Version count
    fn version_count(
        &self,
        package_id: ObjectID,
        object_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let tag = utils::get_type_tag(iota_client, package_id).await?;
            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("version_count").into(),
                vec![tag],
                vec![notarization],
            );

            Ok(ptb.finish())
        }
    }

    /// Created at timestamp
    fn created_at(
        &self,
        package_id: ObjectID,
        object_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            let mut ptb = ProgrammableTransactionBuilder::new();

            let tag = utils::get_type_tag(iota_client, package_id).await?;
            let notarization = {
                let notarization = utils::get_object_ref_by_id(iota_client, object_id).await?;

                ptb.obj(ObjectArg::ImmOrOwnedObject(notarization))
                    .map_err(|e| Error::InvalidArgument(e.to_string()))?
            };

            ptb.programmable_move_call(
                package_id,
                ident_str!("notarization").into(),
                ident_str!("created_at").into(),
                vec![tag],
                vec![notarization],
            );

            Ok(ptb.finish())
        }
    }
}

impl NotarizationOperations for Notarization {}
