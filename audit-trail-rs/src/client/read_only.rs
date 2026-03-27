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
use serde::de::DeserializeOwned;

use super::network_id;
use crate::core::trail::{AuditTrailHandle, AuditTrailReadOnly};
use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;
use crate::package;

/// Optional package ID overrides used when constructing an audit trail client.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PackageOverrides {
    pub audit_trail_package_id: Option<ObjectID>,
    pub tf_components_package_id: Option<ObjectID>,
}

/// A read-only client for interacting with audit trail module objects on a specific network.
#[derive(Clone)]
pub struct AuditTrailClientReadOnly {
    /// The underlying IOTA client adapter used for communication.
    iota_client: IotaClientAdapter,
    /// The [`ObjectID`] of the deployed audit trail package (smart contract).
    audit_trail_pkg_id: ObjectID,
    /// The [`ObjectID`] of the deployed TfComponents package used by audit trails.
    pub(crate) tf_components_pkg_id: ObjectID,
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
        Self::new_internal(client, network, PackageOverrides::default()).await
    }

    async fn new_internal(
        iota_client: IotaClientAdapter,
        network: NetworkName,
        package_overrides: PackageOverrides,
    ) -> Result<Self, Error> {
        let chain_id = network.as_ref().to_string();
        let (network, package_ids) = package::resolve_package_ids(&network, &package_overrides).await?;

        Ok(Self {
            iota_client,
            audit_trail_pkg_id: package_ids.audit_trail_package_id,
            tf_components_pkg_id: package_ids.tf_components_package_id,
            network,
            chain_id,
        })
    }

    /// Creates a new [`AuditTrailClientReadOnly`] with explicit package overrides.
    ///
    /// This function allows overriding the package ID lookup from the registry,
    /// which is useful for local testing or custom deployments where the package
    /// IDs are known ahead of time.
    pub async fn new_with_package_overrides(
        #[cfg(target_arch = "wasm32")] iota_client: WasmIotaClient,
        #[cfg(not(target_arch = "wasm32"))] iota_client: IotaClient,
        package_overrides: PackageOverrides,
    ) -> Result<Self, Error> {
        let client = IotaClientAdapter::new(iota_client);
        let network = network_id(&client).await?;
        Self::new_internal(client, network, package_overrides).await
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

    fn tf_components_package_id(&self) -> Option<ObjectID> {
        Some(self.tf_components_pkg_id)
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
