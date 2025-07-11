// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! A read-only client for interacting with IOTA Notarization module objects.
//!
//! This client provides methods to query the state and metadata of notarized objects
//! on the IOTA network without requiring signing capabilities.

use std::ops::Deref;

#[cfg(not(target_arch = "wasm32"))]
use iota_interaction::IotaClient;
use iota_interaction::IotaClientTrait;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::{ProgrammableTransaction, TransactionKind};
#[cfg(target_arch = "wasm32")]
use iota_interaction_ts::bindings::WasmIotaClient;
use product_common::core_client::CoreClientReadOnly;
use product_common::network_name::NetworkName;
use product_common::package_registry::{Env, Metadata};
use serde::de::DeserializeOwned;

use super::network_id;
use crate::core::move_utils;
use crate::core::operations::{NotarizationImpl, NotarizationOperations};
use crate::core::transactions::get_object_ref_by_id_with_bcs;
use crate::core::types::{Data, LockMetadata, NotarizationMethod, OnChainNotarization, State};
use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;
use crate::package;

/// A read-only client for interacting with IOTA Notarization module objects on a specific network.
///
/// This client allows querying the state and metadata of notarized objects
/// without needing a wallet or signing capabilities. It connects to an IOTA node
/// via the provided `IotaClient` and interacts with the Notarization smart contract
/// deployed on that network.
#[derive(Clone)]
pub struct NotarizationClientReadOnly {
    /// The underlying IOTA client adapter used for communication.
    iota_client: IotaClientAdapter,
    /// The [`ObjectID`] of the deployed Notarization package (smart contract).
    /// All interactions go through this package ID.
    notarization_pkg_id: ObjectID,
    /// The name of the network this client is connected to (e.g., "mainnet", "testnet").
    network: NetworkName,
    chain_id: String,
}

impl Deref for NotarizationClientReadOnly {
    type Target = IotaClientAdapter;
    fn deref(&self) -> &Self::Target {
        &self.iota_client
    }
}

impl NotarizationClientReadOnly {
    /// Returns the name of the network the client is connected to.
    ///
    /// This name is derived from the network ID of the IOTA node the client is connected to.
    /// For the IOTA Mainnet, this will typically be "iota".
    pub const fn network(&self) -> &NetworkName {
        &self.network
    }

    /// Returns the chain identifier for the network this client is connected to.
    ///
    /// This is the raw chain ID string obtained from the IOTA node's network ID.
    /// For the IOTA Mainnet, this will typically be "iota".
    ///
    /// Note: This might be different from the `network()` name if an alias is used.
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    /// Attempts to create a new [`NotarizationClientReadOnly`] from a given IOTA client.
    ///
    /// # Failures
    /// This function fails if the provided `iota_client` is connected to an unrecognized
    /// network for which the notarization package ID is not known in the internal
    /// package registry.
    ///
    /// # Arguments
    ///
    /// * `iota_client`: The IOTA client instance to use for communication. This can be either a native `IotaClient` or
    ///   a WASM-specific `WasmIotaClient`.
    ///
    /// # Returns
    ///
    /// A `Result` containing the initialized [`NotarizationClientReadOnly`] on success,
    /// or an [`Error`] if the network is unrecognized or communication fails.
    pub async fn new(
        #[cfg(target_arch = "wasm32")] iota_client: WasmIotaClient,
        #[cfg(not(target_arch = "wasm32"))] iota_client: IotaClient,
    ) -> Result<Self, Error> {
        let client = IotaClientAdapter::new(iota_client);
        let network = network_id(&client).await?;
        Self::new_internal(client, network).await
    }

    /// Internal helper function to create a new [`NotarizationClientReadOnly`].
    ///
    /// This function looks up the notarization package ID based on the provided network name
    /// using the internal package registry.
    ///
    /// # Arguments
    ///
    /// * `iota_client`: The IOTA client adapter.
    /// * `network`: The name of the network.
    async fn new_internal(iota_client: IotaClientAdapter, network: NetworkName) -> Result<Self, Error> {
        let chain_id = network.as_ref().to_string();
        let (network, notarization_pkg_id) = {
            let package_registry = package::notarization_package_registry().await;
            let package_id = package_registry
        .package_id(&network)
        .ok_or_else(|| {
        Error::InvalidConfig(format!(
            "no information for a published `notarization` package on network {network}; try to use `NotarizationClientReadOnly::new_with_package_id`"
            ))
        })?;
            let network = match chain_id.as_str() {
                product_common::package_registry::MAINNET_CHAIN_ID => {
                    NetworkName::try_from("iota").expect("valid network name")
                }
                _ => package_registry
                    .chain_alias(&chain_id)
                    .and_then(|alias| NetworkName::try_from(alias).ok())
                    .unwrap_or(network),
            };

            (network, package_id)
        };
        Ok(NotarizationClientReadOnly {
            iota_client,
            notarization_pkg_id,
            network,
            chain_id,
        })
    }

    /// Creates a new [`NotarizationClientReadOnly`] with a specific notarization package ID.
    ///
    /// This function allows overriding the package ID lookup from the registry, which is useful
    /// for connecting to networks where the package ID is known but not yet registered, or
    /// for testing with custom deployments.
    ///
    /// # Arguments
    ///
    /// * `iota_client`: The IOTA client instance.
    /// * `package_id`: The specific [`ObjectID`] of the Notarization package to use.
    ///
    /// # Returns
    /// A `Result` containing the initialized [`NotarizationClientReadOnly`] or an [`Error`].
    pub async fn new_with_pkg_id(
        #[cfg(target_arch = "wasm32")] iota_client: WasmIotaClient,
        #[cfg(not(target_arch = "wasm32"))] iota_client: IotaClient,
        package_id: ObjectID,
    ) -> Result<Self, Error> {
        let client = IotaClientAdapter::new(iota_client);
        let network = network_id(&client).await?;

        // Use the passed pkg_id to add a new env or override the information of an existing one.
        {
            let mut registry = package::notarization_package_registry_mut().await;
            registry.insert_env(Env::new(network.as_ref()), Metadata::from_package_id(package_id));
        }

        Self::new_internal(client, network).await
    }

    /// Retrieves the [`OnChainNotarization`] of a notarized object.
    ///
    /// This method returns the on-chain notarization object for the given object ID.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing the [`OnChainNotarization`] or an [`Error`].
    pub async fn get_notarization_by_id(&self, notarized_object_id: ObjectID) -> Result<OnChainNotarization, Error> {
        let notarization_object = get_object_ref_by_id_with_bcs(self, &notarized_object_id).await?;

        Ok(notarization_object)
    }

    /// Retrieves the `last_state_change_at` timestamp of a notarized object.
    ///
    /// This timestamp indicates the time of the most recent state change for the object.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing the timestamp as a `u64` or an [`Error`].
    pub async fn last_state_change_ts(&self, notarized_object_id: ObjectID) -> Result<u64, Error> {
        let tx = NotarizationImpl::last_change_ts(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `created_at` timestamp of a notarized object.
    ///
    /// This timestamp indicates when the notarized object was initially created.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing the timestamp as a `u64` or an [`Error`].
    pub async fn created_at_ts(&self, notarized_object_id: ObjectID) -> Result<u64, Error> {
        let tx = NotarizationImpl::created_at(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `state_version_count` of a notarization object by its `object_id`.
    ///
    /// This count represents the number of times the object's state has been updated.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing the version count as a `u64` or an [`Error`].
    pub async fn state_version_count(&self, notarized_object_id: ObjectID) -> Result<u64, Error> {
        let tx = NotarizationImpl::version_count(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `description` of a notarization object by its `object_id`.
    ///
    /// The description is an optional string associated with the object.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing an `Option<String>` or an [`Error`]. `None` if no description is set.
    pub async fn description(&self, notarized_object_id: ObjectID) -> Result<Option<String>, Error> {
        let tx = NotarizationImpl::description(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `updatable_metadata` of a notarization object by its `object_id`.
    ///
    /// This metadata is an optional string that can be updated after creation.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing an `Option<String>` or an [`Error`]. `None` if no updatable metadata is set.
    pub async fn updatable_metadata(&self, notarized_object_id: ObjectID) -> Result<Option<String>, Error> {
        let tx = NotarizationImpl::updatable_metadata(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `notarization_method` of a notarization object by its `object_id`.
    ///
    /// This indicates the method used for notarizing the object's state changes.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing the [`NotarizationMethod`] or an [`Error`].
    pub async fn notarization_method(&self, notarized_object_id: ObjectID) -> Result<NotarizationMethod, Error> {
        let tx = NotarizationImpl::notarization_method(notarized_object_id, self).await?;
        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `lock_metadata` of a notarization object by its `object_id`.
    ///
    /// This metadata contains information about any locks applied to the object (update, destroy, transfer).
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing an `Option<LockMetadata>` or an [`Error`]. `None` if no locks are set.
    pub async fn lock_metadata(&self, notarized_object_id: ObjectID) -> Result<Option<LockMetadata>, Error> {
        let tx = NotarizationImpl::lock_metadata(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `state` of a notarization object by its `object_id`.
    ///
    /// This method specifically handles notarized objects with **default state types only**
    /// (`Vec<u8>` or `String`). The notarization system guarantees that objects accessed
    /// through this method contain state data of these primitive types.
    ///
    /// For custom types, use [`Self::state_as`] which provides type-safe deserialization
    /// without the default type constraints.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing the [`State<Data>`] or an [`Error`].
    pub async fn state(&self, notarized_object_id: ObjectID) -> Result<State, Error> {
        let type_tag = move_utils::get_type_tag(self, &notarized_object_id).await?;
        let type_str = type_tag.to_string();

        let tx = NotarizationImpl::state(notarized_object_id, self).await?;

        if type_str == "vector<u8>" {
            let state: State<Vec<u8>> = self.execute_read_only_transaction(tx).await?;
            Ok(State {
                data: Data::Bytes(state.data),
                metadata: state.metadata,
            })
        } else if type_str.contains("::string::String") {
            let state: State<String> = self.execute_read_only_transaction(tx).await?;
            Ok(State {
                data: Data::Text(state.data),
                metadata: state.metadata,
            })
        } else {
            return Err(Error::InvalidArgument(format!("Unsupported state type: {type_str}")));
        }
    }

    /// Retrieves the `state` of a notarization object by its `object_id` and deserializes it into a custom type `T`.
    /// This method is useful when the state data is of a custom type.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing the [`State<T>`] or an [`Error`].
    pub async fn state_as<T: DeserializeOwned>(&self, notarized_object_id: ObjectID) -> Result<State<T>, Error> {
        let tx = NotarizationImpl::state(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Checks if the notarized object is currently locked against state updates.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing `true` if the object is update-locked, `false` otherwise, or an [`Error`].
    pub async fn is_update_locked(&self, notarized_object_id: ObjectID) -> Result<bool, Error> {
        let tx = NotarizationImpl::is_update_locked(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Checks if the notarized object is currently allowed to be destroyed.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing `true` if the object is destroy-allowed, `false` otherwise, or an [`Error`].
    pub async fn is_destroy_allowed(&self, notarized_object_id: ObjectID) -> Result<bool, Error> {
        let tx = NotarizationImpl::is_destroy_allowed(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Checks if the notarized object is currently locked against transfer.
    ///
    /// # Arguments
    ///
    /// * `notarized_object_id`: The [`ObjectID`] of the notarized object.
    ///
    /// # Returns
    /// A `Result` containing `true` if the object is transfer-locked, `false` otherwise, or an [`Error`].
    pub async fn is_transfer_locked(&self, notarized_object_id: ObjectID) -> Result<bool, Error> {
        let tx = NotarizationImpl::is_transfer_locked(notarized_object_id, self).await?;

        self.execute_read_only_transaction(tx).await
    }
}

impl NotarizationClientReadOnly {
    /// A helper function to execute a read-only transaction and deserialize
    /// the result into the specified type `T`.
    ///
    /// This function uses the `dev_inspect_transaction_block` endpoint of the IOTA client
    /// to simulate the execution of a programmable transaction without submitting it
    /// to the network. The result of the first return value of the first execution result
    /// is deserialized using BCS.
    ///
    /// # Arguments
    ///
    /// * `tx`: The [`ProgrammableTransaction`] to execute.
    ///
    /// # Returns
    /// A `Result` containing the deserialized result of type `T` or an [`Error`].
    async fn execute_read_only_transaction<T: DeserializeOwned>(
        &self,
        tx: ProgrammableTransaction,
    ) -> Result<T, Error> {
        let inspection_result = self
            .iota_client
            .read_api()
            .dev_inspect_transaction_block(IotaAddress::ZERO, TransactionKind::programmable(tx), None, None, None)
            .await
            .map_err(|err| Error::UnexpectedApiResponse(format!("Failed to inspect transaction block: {err}")))?;

        let execution_results = inspection_result
            .results
            .ok_or_else(|| Error::UnexpectedApiResponse("DevInspectResults missing 'results' field".to_string()))?;

        let (return_value_bytes, _) = execution_results
            .first()
            .ok_or_else(|| Error::UnexpectedApiResponse("Execution results list is empty".to_string()))?
            .return_values
            .first()
            .ok_or_else(|| Error::InvalidArgument("should have at least one return value".to_string()))?;

        let deserialized_output = bcs::from_bytes::<T>(return_value_bytes)?;

        Ok(deserialized_output)
    }
}

#[async_trait::async_trait]
impl CoreClientReadOnly for NotarizationClientReadOnly {
    /// Returns the [`ObjectID`] of the Notarization package used by this client.
    fn package_id(&self) -> ObjectID {
        self.notarization_pkg_id
    }

    /// Returns the name of the network the client is connected to.
    ///
    /// This is part of the [`CoreClientReadOnly`] trait implementation.
    fn network_name(&self) -> &NetworkName {
        &self.network
    }

    /// Returns a reference to the underlying [`IotaClientAdapter`].
    ///
    /// This is part of the [`CoreClientReadOnly`] trait implementation.
    fn client_adapter(&self) -> &IotaClientAdapter {
        &self.iota_client
    }
}
