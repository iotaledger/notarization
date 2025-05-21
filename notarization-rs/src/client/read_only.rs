// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::ops::Deref;

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::{ProgrammableTransaction, TransactionKind};
use iota_interaction::{IotaClient, IotaClientTrait};
use product_common::core_client::CoreClientReadOnly;
use product_common::network_name::NetworkName;
use product_common::package_registry::{Env, Metadata};
use serde::de::DeserializeOwned;

use crate::client_tools::network_id;
use crate::core::operations::{NotarizationImpl, NotarizationOperations};
use crate::core::state::{Data, State};
use crate::core::timelock::LockMetadata;
use crate::core::NotarizationMethod;
use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;
use crate::package;
use crate::package::MAINNET_CHAIN_ID;

/// A read-only client for interacting with IOTA Notarization module objects.
#[derive(Clone)]
pub struct NotarizationClientReadOnly {
    iota_client: IotaClientAdapter,
    notarization_pkg_id: ObjectID,
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
    /// Returns the name of the network the client is currently connected to.
    pub const fn network(&self) -> &NetworkName {
        &self.network
    }

    /// Returns the chain identifier for the network this client is connected to.
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    /// Attempts to create a new [`NotarizationClientReadOnly`] from a given IOTA client.
    ///
    /// # Failures
    /// This function fails if the provided `iota_client` is connected to an unrecognized
    /// network for which the notarization package ID is not known.
    pub async fn new(
        #[cfg(target_arch = "wasm32")] iota_client: WasmIotaClient,
        #[cfg(not(target_arch = "wasm32"))] iota_client: IotaClient,
    ) -> Result<Self, Error> {
        let client = IotaClientAdapter::new(iota_client.into());
        let network = network_id(&client).await?;
        Self::new_internal(client, network).await
    }

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
                // Replace Mainnet's name with "iota".
                MAINNET_CHAIN_ID => NetworkName::try_from("iota").expect("valid network name"),
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
    pub async fn new_with_pkg_id(
        #[cfg(target_arch = "wasm32")] iota_client: WasmIotaClient,
        #[cfg(not(target_arch = "wasm32"))] iota_client: IotaClient,
        package_id: ObjectID,
    ) -> Result<Self, Error> {
        let client = IotaClientAdapter::new(iota_client.into());
        let network = network_id(&client).await?;

        // Use the passed pkg_id to add a new env or override the information of an existing one.
        {
            let mut registry = package::notarization_package_registry_mut().await;
            registry.insert_env(Env::new(network.as_ref()), Metadata::from_package_id(package_id));
        }

        Self::new_internal(client, network).await
    }

    /// Retrieves the `last_state_change_at` timestamp of a notarization object by its `object_id`.
    pub async fn last_state_change_ts(&self, notarized_object_id: ObjectID) -> Result<u64, Error> {
        let tx =
            NotarizationImpl::last_change_ts(self.notarization_pkg_id, notarized_object_id, &self.iota_client).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `created_at` timestamp of a notarization object by its `object_id`.
    pub async fn created_at_ts(&self, notarized_object_id: ObjectID) -> Result<u64, Error> {
        let tx = NotarizationImpl::created_at(self.notarization_pkg_id, notarized_object_id, &self.iota_client).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `state_version_count` of a notarization object by its `object_id`.
    pub async fn state_version_count(&self, notarized_object_id: ObjectID) -> Result<u64, Error> {
        let tx =
            NotarizationImpl::version_count(self.notarization_pkg_id, notarized_object_id, &self.iota_client).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `description` of a notarization object by its `object_id`.
    pub async fn description(&self, notarized_object_id: ObjectID) -> Result<Option<String>, Error> {
        let tx =
            NotarizationImpl::description(self.notarization_pkg_id, notarized_object_id, &self.iota_client).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `updateable_metadata` of a notarization object by its `object_id`.
    pub async fn updateable_metadata(&self, notarized_object_id: ObjectID) -> Result<Option<String>, Error> {
        let tx =
            NotarizationImpl::updateable_metadata(self.notarization_pkg_id, notarized_object_id, &self.iota_client)
                .await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `notarization_method` of a notarization object by its `object_id`.
    pub async fn notarization_method(&self, notarized_object_id: ObjectID) -> Result<NotarizationMethod, Error> {
        let tx =
            NotarizationImpl::notarization_method(self.notarization_pkg_id, notarized_object_id, &self.iota_client)
                .await?;
        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `lock_metadata` of a notarization object by its `object_id`.
    pub async fn lock_metadata(&self, notarized_object_id: ObjectID) -> Result<Option<LockMetadata>, Error> {
        let tx =
            NotarizationImpl::lock_metadata(self.notarization_pkg_id, notarized_object_id, &self.iota_client).await?;

        self.execute_read_only_transaction(tx).await
    }

    /// Retrieves the `state` of a notarization object by its `object_id`.
    ///
    /// This method assumes the state data is of the default `Data` type (Vec<u8> or String).
    pub async fn state(&self, notarized_object_id: ObjectID) -> Result<State<Data>, Error> {
        self.state_as::<Data>(notarized_object_id).await
    }

    /// Retrieves the `state` of a notarization object by its `object_id` and deserializes it into the specified type
    /// `T`.

    pub async fn state_as<T: DeserializeOwned>(&self, notarized_object_id: ObjectID) -> Result<State<T>, Error> {
        let tx = NotarizationImpl::state(self.notarization_pkg_id, notarized_object_id, &self.iota_client).await?;

        self.execute_read_only_transaction(tx).await
    }

    pub async fn is_update_locked(&self, notarized_object_id: ObjectID) -> Result<bool, Error> {
        let tx = NotarizationImpl::is_update_locked(self.notarization_pkg_id, notarized_object_id, &self.iota_client)
            .await?;

        self.execute_read_only_transaction(tx).await
    }

    pub async fn is_destroy_locked(&self, notarized_object_id: ObjectID) -> Result<bool, Error> {
        let tx = NotarizationImpl::is_destroy_locked(self.notarization_pkg_id, notarized_object_id, &self.iota_client)
            .await?;

        self.execute_read_only_transaction(tx).await
    }

    pub async fn is_transfer_locked(&self, notarized_object_id: ObjectID) -> Result<bool, Error> {
        let tx = NotarizationImpl::is_transfer_locked(self.notarization_pkg_id, notarized_object_id, &self.iota_client)
            .await?;

        self.execute_read_only_transaction(tx).await
    }
}

impl NotarizationClientReadOnly {
    /// A helper function to execute a read-only transaction and deserialize
    /// the result into the specified type `T`.
    async fn execute_read_only_transaction<T: DeserializeOwned>(
        &self,
        tx: ProgrammableTransaction,
    ) -> Result<T, Error> {
        let result = self
            .iota_client
            .read_api()
            .dev_inspect_transaction_block(IotaAddress::ZERO, TransactionKind::programmable(tx), None, None, None)
            .await
            .expect("Failed to inspect transaction");

        let execution_results = result.results.expect("should have results");

        let (result, _) = execution_results
            .first()
            .expect("should have at least one result")
            .return_values
            .first()
            .expect("should have at least one return value");

        let result = bcs::from_bytes::<T>(result).expect("should be a result");

        Ok(result)
    }
}

#[async_trait::async_trait]
impl CoreClientReadOnly for NotarizationClientReadOnly {
    fn package_id(&self) -> ObjectID {
        self.notarization_pkg_id
    }
    fn network_name(&self) -> &NetworkName {
        &self.network
    }
    fn client_adapter(&self) -> &IotaClientAdapter {
        &self.iota_client
    }
}
