// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use product_common::network_name::NetworkName;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::crypto::PublicKey;
#[cfg(not(target_arch = "wasm32"))]
use iota_interaction::IotaClient;
use iota_interaction::IotaKeySignature;
#[cfg(target_arch = "wasm32")]
use iota_interaction_ts::bindings::WasmIotaClient;
use secret_storage::Signer;

use crate::client_tools::network_id;
use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;
use crate::well_known_networks::network_metadata;

/// Indicates the used Notarization method.
pub enum NotarizationMethod {
    Dynamic,
    Locked,
}

/// Account of a user that uses an IOTA client to execute transactions.
/// Is not needed for read access.
#[derive(Clone)]
pub struct ClientAccount<S: Signer<IotaKeySignature>> {
    /// The signer of the client.
    pub signer: S,
    /// The address of the client account.
    pub address: IotaAddress,
    /// The public key of the client account.
    pub public_key: PublicKey,
}

impl<S: Signer<IotaKeySignature>> ClientAccount<S> {
    pub async fn new(signer: S) -> Result<Self> {
        let public_key = signer
            .public_key()
            .await
            .map_err(|e| Error::InvalidKey(e.to_string()))?;
        let address = IotaAddress::from(&public_key);

        Ok(ClientAccount {
            signer,
            address,
            public_key,
        })
    }
}

/// Facilitates creating a [`Notarization`] instance
#[derive(Clone)]
pub struct NotarizationBuilder<S: Signer<IotaKeySignature>> {
    iota_client: IotaClientAdapter,
    iota_notarization_pkg_id: ObjectID,
    network: NetworkName,
    signer: Option<S>,
}

impl<S: Signer<IotaKeySignature>> NotarizationBuilder<S> {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            /// Create a new [`NotarizationBuilder`] from a given [`IotaClient`].
            ///
            /// # Failures
            /// This function fails if the provided `iota_client` is connected to an unrecognized
            /// network.
            ///
            /// # Notes
            /// When trying to connect to a local or unofficial network prefer using
            /// [`NotarizationBuilder::new_with_pkg_id`].
            pub async fn new(iota_client: WasmIotaClient) -> Result<Self, Error> {
                Self::new_internal(IotaClientAdapter::new(iota_client)?).await
            }

            /// Create a new [`NotarizationBuilder`] from the given [`IotaClient`] and uses
            /// the Move code published to the specified iota_notarization_pkg_id.
            pub async fn new_with_pkg_id(iota_client: WasmIotaClient, iota_notarization_pkg_id: ObjectID) -> Result<Self, Error> {
                Self::new_with_pkg_id_internal(
                    IotaClientAdapter::new(iota_client)?,
                    iota_notarization_pkg_id
                ).await
            }
        } else {
            /// Attempts to create a new [`NotarizationBuilder`] from a given [`IotaClient`].
            ///
            /// # Failures
            /// This function fails if the provided `iota_client` is connected to an unrecognized
            /// network.
            ///
            /// # Notes
            /// When trying to connect to a local or unofficial network prefer using
            /// [`NotarizationBuilder::new_with_pkg_id`].
            pub async fn new(iota_client: IotaClient) -> Result<Self, Error> {
                Self::new_internal(IotaClientAdapter::new(iota_client).map_err(|e| Error::IotaClient(e))?).await
            }

            /// Create a new [`NotarizationBuilder`] from the given [`IotaClient`] and uses
            /// the Move code published to the specified iota_notarization_pkg_id.
            pub async fn new_with_pkg_id(iota_client: IotaClient, iota_notarization_pkg_id: ObjectID) -> Result<Self, Error> {
                Self::new_with_pkg_id_internal(
                    IotaClientAdapter::new(iota_client).map_err(|e| Error::IotaClient(e))?,
                    iota_notarization_pkg_id
                ).await
            }
        }
    }

    pub async fn create_dynamic(self) -> Result<Notarization<S>> {
        self.create(NotarizationMethod::Dynamic).await
    }

    pub async fn create_locked(self) -> Result<Notarization<S>> {
        self.create(NotarizationMethod::Locked).await
    }

    pub async fn create(self, method: NotarizationMethod) -> Result<Notarization<S>> {
        let account = if let Some(signer) = self.signer {
            Some(ClientAccount::new(signer).await?)
        } else {
            None
        };

        let iota_notarization_pkg_id = ObjectID::from_hex_literal("0x00")?;

        // TODO: Create and execute the PTB to create the notarization object on the ledger
        let notarization_id = ObjectID::from_hex_literal("0x00")?;

        Ok(Notarization {
            iota_client: self.iota_client,
            iota_notarization_pkg_id,
            notarization_id,
            account,
        })
    }

    async fn new_internal(iota_client: IotaClientAdapter) -> Result<Self, Error> {
        let network = network_id(&iota_client).await?;
        let metadata = network_metadata(&network).ok_or_else(|| {
            Error::InvalidConfig(format!(
                "unrecognized network \"{network}\". Use `new_with_pkg_id` instead."
            ))
        })?;
        // If the network has a well known alias use it otherwise default to the network's chain ID.
        let network = metadata.network_alias().unwrap_or(network);

        let pkg_id = metadata.latest_pkg_id();

        Ok(NotarizationBuilder {
            iota_client,
            iota_notarization_pkg_id: pkg_id,
            network,
            signer: None,
        })
    }

    async fn new_with_pkg_id_internal(
        iota_client: IotaClientAdapter,
        iota_notarization_pkg_id: ObjectID,
    ) -> Result<Self, Error> {
        let network = network_id(&iota_client).await?;
        Ok(NotarizationBuilder {
            iota_client,
            iota_notarization_pkg_id,
            network,
            signer: None,
        })
    }

    pub fn signer(mut self, signer: S) -> Self {
        self.signer = Some(signer);
        self
    }
}

pub enum StateData {
    Binary(Vec<u8>),
    String(String),
}

/// Manages an existing Notarization object stored on the ledger
#[derive(Clone)]
pub struct Notarization<S: Signer<IotaKeySignature>> {
    iota_client: IotaClientAdapter,
    /// Package ID of the used Notarization smart contract
    iota_notarization_pkg_id: ObjectID,
    /// References the managed notarization object stored on the ledger
    notarization_id: ObjectID,
    /// The client account of the user, only needed for write access
    account: Option<ClientAccount<S>>,
}

impl<S: Signer<IotaKeySignature>> Notarization<S> {
    pub fn signer(&self) -> Option<&S> {
        self.account.as_ref().map(|account| &account.signer)
    }

    pub async fn new(
        iota_client: IotaClientAdapter,
        iota_notarization_pkg_id: ObjectID,
        notarization_id: ObjectID,
        account: Option<ClientAccount<S>>,
    ) -> Result<Self> {
        Ok(Notarization {
            iota_client,
            iota_notarization_pkg_id,
            notarization_id,
            account,
        })
    }

    pub fn update_state<D>(&mut self, data: StateData, _metadata: Option<String>) -> Result<()> {
        // ....

        match data {
            StateData::Binary(_data) => {
                unimplemented!("Create a new Notarization<vec<u8>> object on the ledger")
            }
            StateData::String(_) => {
                unimplemented!("Create a new Notarization<String> object on the ledger")
            }
        }
    }
}