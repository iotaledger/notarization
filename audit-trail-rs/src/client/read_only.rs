// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! A read-only client for interacting with IOTA Audit Trail module objects.

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
use product_common::package_registry::Env;
use serde::de::DeserializeOwned;

use super::network_id;
use crate::core::handler::{AuditTrailHandle, AuditTrailReadOnly};
use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;
use crate::package;

/// A read-only client for interacting with audit trail module objects on a specific network.
#[derive(Clone)]
pub struct AuditTrailClientReadOnly {
    /// The underlying IOTA client adapter used for communication.
    iota_client: IotaClientAdapter,
    /// The [`ObjectID`] of the deployed audit trail package (smart contract).
    audit_trail_pkg_id: ObjectID,
    /// The name of the network this client is connected to (e.g., "mainnet", "testnet").
    network: NetworkName,
    /// Raw chain identifier returned by the IOTA node.
    chain_id: String,
}

impl Deref for AuditTrailClientReadOnly {
    type Target = IotaClientAdapter;
    fn deref(&self) -> &Self::Target {
        &self.iota_client
    }
}

impl AuditTrailClientReadOnly {
    /// Returns the name of the network the client is connected to.
    pub const fn network(&self) -> &NetworkName {
        &self.network
    }

    /// Returns the raw chain identifier for the network this client is connected to.
    pub fn chain_id(&self) -> &str {
        &self.chain_id
    }

    /// Returns the package ID used by this client.
    pub fn package_id(&self) -> ObjectID {
        self.audit_trail_pkg_id
    }

    /// Returns a reference to the underlying IOTA client adapter.
    pub const fn iota_client(&self) -> &IotaClientAdapter {
        &self.iota_client
    }

    /// Returns a typed handle bound to a trail id.
    pub fn trail<'a>(&'a self, trail_id: ObjectID) -> AuditTrailHandle<'a, Self> {
        AuditTrailHandle::new(self, trail_id)
    }

    /// Attempts to create a new [`AuditTrailClientReadOnly`] from a given IOTA client.
    ///
    /// This resolves the package ID from the internal registry based on the network.
    pub async fn new(
        #[cfg(target_arch = "wasm32")] iota_client: WasmIotaClient,
        #[cfg(not(target_arch = "wasm32"))] iota_client: IotaClient,
    ) -> Result<Self, Error> {
        let client = IotaClientAdapter::new(iota_client);
        let network = network_id(&client).await?;
        Self::new_internal(client, network).await
    }

    async fn new_internal(iota_client: IotaClientAdapter, network: NetworkName) -> Result<Self, Error> {
        let chain_id = network.as_ref().to_string();
        let (network, audit_trail_pkg_id) = {
            let package_registry = package::audit_trail_package_registry().await;
            let package_id = package_registry
                .package_id(&network)
                .ok_or_else(|| {
                    Error::InvalidConfig(format!(
                        "no information for a published `audit_trail` package on network {network}; try to use `AuditTrailClientReadOnly::new_with_pkg_id`"
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

        Ok(Self {
            iota_client,
            audit_trail_pkg_id,
            network,
            chain_id,
        })
    }

    /// Creates a new [`AuditTrailClientReadOnly`] with a specific audit trail package ID.
    ///
    /// This function allows overriding the package ID lookup from the
    /// registry, which is useful for connecting to networks where the package
    /// ID is known but not yet registered, or for testing with custom deployments.
    pub async fn new_with_pkg_id(
        #[cfg(target_arch = "wasm32")] iota_client: WasmIotaClient,
        #[cfg(not(target_arch = "wasm32"))] iota_client: IotaClient,
        package_id: ObjectID,
    ) -> Result<Self, Error> {
        let client = IotaClientAdapter::new(iota_client);
        let network = network_id(&client).await?;

        {
            let mut registry = package::audit_trail_package_registry_mut().await;
            registry.insert_env_history(Env::new(network.as_ref()), vec![package_id]);
        }

        Self::new_internal(client, network).await
    }
}

#[async_trait::async_trait]
impl CoreClientReadOnly for AuditTrailClientReadOnly {
    fn package_id(&self) -> ObjectID {
        self.audit_trail_pkg_id
    }

    fn network_name(&self) -> &NetworkName {
        &self.network
    }

    fn client_adapter(&self) -> &IotaClientAdapter {
        &self.iota_client
    }
}

#[async_trait::async_trait]
impl AuditTrailReadOnly for AuditTrailClientReadOnly {
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
