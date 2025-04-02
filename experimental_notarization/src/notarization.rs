// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use anyhow::Result;
use fastcrypto::ed25519::Ed25519PublicKey;
use fastcrypto::traits::ToFromBytes;
use identity_iota_interaction::types::base_types::IotaAddress;
use identity_iota_interaction::types::base_types::ObjectID;
use identity_iota_interaction::types::crypto::PublicKey;
use identity_iota_interaction::IotaKeySignature;
use iota_sdk::IotaClient;
use secret_storage::Signer;

/// Facilitates creating a [`Notarization`] instance
pub struct NotarizationBuilder<S: Signer<IotaKeySignature>> {
    iota_client: IotaClient,
    signer: Option<S>,
}

impl<S: Signer<IotaKeySignature>> NotarizationBuilder<S> {
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
}

/// Manages an existing Notarization object stored on the ledger
#[derive(Clone)]
pub struct Notarization {
    iota_client: IotaClient,
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
