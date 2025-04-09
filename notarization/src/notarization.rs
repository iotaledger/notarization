// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0


use anyhow::Result;

use fastcrypto::ed25519::Ed25519PublicKey;
use fastcrypto::traits::ToFromBytes;

use secret_storage::Signer;

use identity_iota_core::NetworkName;

use identity_iota_interaction::types::base_types::IotaAddress;
use identity_iota_interaction::types::base_types::ObjectID;
use identity_iota_interaction::types::crypto::PublicKey;
use identity_iota_interaction::IotaKeySignature;

use crate::client_tools::network_id;
use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;
use crate::well_known_networks::network_metadata;

#[cfg(not(target_arch = "wasm32"))]
use identity_iota_interaction::IotaClient;

#[cfg(target_arch = "wasm32")]
use iota_interaction_ts::bindings::WasmIotaClient;

/// Facilitates creating a [`Notarization`] instance
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
          Self::new_internal(IotaClientAdapter::new(iota_client)?).await
        }
      }
    }

    pub async fn finish(self) -> Result<Notarization> {
        let Some(signer) = self.signer else {
            anyhow::bail!("Signer is not set")
        };
        let public_key = signer
            .public_key()
            .await
            .map_err(|e| Error::InvalidKey(e.to_string()))?;
        let address = convert_to_address(public_key.as_ref())?;

        // TODO: Create PTB to create and the notarization object on the ledger
        let iota_notarization_pkg_id = ObjectID::from_hex_literal("0x00")?;
        Ok(Notarization {
            iota_client: self.iota_client,
            iota_notarization_pkg_id,
            address,
            public_key,
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
}

pub enum StateData {
    Binary(Vec<u8>),
    String(String),
}

/// Manages an existing Notarization object stored on the ledger
#[derive(Clone)]
pub struct Notarization {
    iota_client: IotaClientAdapter,
    /// References the managed notarization object sored on the ledger
    iota_notarization_pkg_id: ObjectID,

    /// The address of the client.
    address: IotaAddress,
    /// The public key of the client.
    public_key: PublicKey,
}

impl Notarization {
    // pub async fn new_from_ledger(iota_client: IotaClient, notarization_id: ObjectID) -> Result<Self> {
    //   let mut notarization = Self::new(iota_client);
    //   notarization.iota_notarization_pkg_id = Some(notarization_id);
    //   notarization
    // }
    //
    // pub fn is_stored_on_ledger(&self) -> bool {
    //   self.iota_notarization_pkg_id.is_some()
    // }
    //
    // pub fn create() {
    //
    // }

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

/// Utility function to convert a public key's bytes into an [`IotaAddress`].
pub fn convert_to_address(sender_public_key: &[u8]) -> Result<IotaAddress, Error> {
    let public_key = Ed25519PublicKey::from_bytes(sender_public_key).map_err(|err| {
        Error::InvalidKey(format!(
            "could not parse public key to Ed25519 public key; {err}"
        ))
    })?;

    Ok(IotaAddress::from(&public_key))
}
