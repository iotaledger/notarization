// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::future::Future;
use std::str::FromStr;

use iota_interaction_rust::IotaClientAdapter;
use iota_interaction::ident_str;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::{Argument, ObjectArg, ProgrammableTransaction};
use iota_interaction::types::Identifier;

use super::utils;
use crate::core::state::State;
use crate::core::timelock::TimeLock;
use crate::error::Error;

#[derive(Debug, Clone)]
/// A unified notarization type that can be either dynamic or locked
pub struct Notarization;

impl Notarization {
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
    async fn build_transaction<F>(
        iota_client: &IotaClientAdapter,
        package_id: ObjectID,
        object_id: ObjectID,
        method: impl AsRef<str>,
        additional_args: F,
    ) -> Result<ProgrammableTransaction, Error>
    where
        F: FnOnce(&mut ProgrammableTransactionBuilder) -> Result<Vec<Argument>, Error>,
    {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let tag = vec![utils::get_type_tag(iota_client, &object_id).await?];

        let mut args = {
            let notarization = utils::get_object_ref_by_id(iota_client, &object_id).await?;
            vec![ptb
                .obj(ObjectArg::ImmOrOwnedObject(notarization))
                .map_err(|e| {
                    Error::InvalidArgument(format!("Failed to create object argument: {}", e))
                })?]
        };
        // Add additional arguments
        args.extend(additional_args(&mut ptb).map_err(|e| {
            Error::InvalidArgument(format!("Failed to add additional arguments: {}", e))
        })?);

        // Create method identifier
        let method_id = Identifier::from_str(method.as_ref()).map_err(|e| {
            Error::InvalidArgument(format!("Invalid method name '{}': {}", method.as_ref(), e))
        })?;

        // Build the move call
        ptb.programmable_move_call(
            package_id,
            ident_str!("notarization").into(),
            method_id,
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
pub trait NotarizationOperations {
    /// Build a transaction that creates a new locked notarization
    fn new_locked(
        &self,
        package_id: ObjectID,
        state: State,
        immutable_description: Option<String>,
        updateable_metadata: Option<String>,
        delete_lock: TimeLock,
    ) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let tag = state.data.tag();
        let clock = utils::get_clock_ref(&mut ptb);
        let state_arg = state.into_ptb(&mut ptb, package_id)?;
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
        package_id: ObjectID,
        state: State,
        immutable_description: Option<String>,
        updateable_metadata: Option<String>,
        transfer_lock: Option<TimeLock>,
    ) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let tag = state.data.tag();
        let clock = utils::get_clock_ref(&mut ptb);
        let state_arg = state.into_ptb(&mut ptb, package_id)?;
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
        package_id: ObjectID,
        object_id: ObjectID,
        new_state: State,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "update_state",
                |ptb| {
                    Ok(vec![
                        utils::get_clock_ref(ptb),
                        new_state.into_ptb(ptb, package_id)?,
                    ])
                },
            )
            .await
        }
    }

    /// Build a transaction that destroys a notarization
    fn destroy(
        &self,
        iota_client: &IotaClientAdapter,
        package_id: ObjectID,
        object_id: ObjectID,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            Notarization::build_transaction(iota_client, package_id, object_id, "destroy", |ptb| {
                Ok(vec![utils::get_clock_ref(ptb)])
            })
            .await
        }
    }

    /// Build a transaction that updates the metadata of a notarization
    fn update_metadata(
        &self,
        iota_client: &IotaClientAdapter,
        package_id: ObjectID,
        object_id: ObjectID,
        new_metadata: Option<String>,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "update_metadata",
                |ptb| {
                    Ok(vec![
                        utils::get_clock_ref(ptb),
                        utils::new_move_option_string(new_metadata, ptb)?,
                    ])
                },
            )
            .await
        }
    }

    /// Build a transaction that returns the notarization method
    fn notarization_method(
        &self,
        package_id: ObjectID,
        object_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "notarization_method",
                |_| Ok(vec![]),
            )
            .await
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
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "is_update_locked",
                |ptb| Ok(vec![utils::get_clock_ref(ptb)]),
            )
            .await
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
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "is_destroy_locked",
                |ptb| Ok(vec![utils::get_clock_ref(ptb)]),
            )
            .await
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
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "is_transfer_locked",
                |ptb| Ok(vec![utils::get_clock_ref(ptb)]),
            )
            .await
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
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "is_destroy_allowed",
                |ptb| Ok(vec![utils::get_clock_ref(ptb)]),
            )
            .await
        }
    }

    /// Last change timestamp
    fn last_change_ts(
        &self,
        package_id: ObjectID,
        object_id: ObjectID,
        iota_client: &IotaClientAdapter,
    ) -> impl Future<Output = Result<ProgrammableTransaction, Error>> + Send {
        async move {
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "last_change",
                |_| Ok(vec![]),
            )
            .await
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
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "version_count",
                |_| Ok(vec![]),
            )
            .await
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
            Notarization::build_transaction(
                iota_client,
                package_id,
                object_id,
                "created_at",
                |_| Ok(vec![]),
            )
            .await
        }
    }
}

impl NotarizationOperations for Notarization {}
