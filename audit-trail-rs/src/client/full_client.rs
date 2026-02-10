// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! A full client wrapper for audit trail interactions.
//!
//! This client includes signing capabilities for executing transactions.

use std::ops::Deref;

use crate::client::read_only::AuditTrailClientReadOnly;
use crate::core::builder::AuditTrailBuilder;
use crate::core::trail::{AuditTrailFull, AuditTrailHandle, AuditTrailReadOnly};
use crate::error::Error;
use async_trait::async_trait;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaKeySignature, OptionalSync};
use iota_interaction_rust::IotaClientAdapter;
use iota_sdk::types::base_types::IotaAddress;
use iota_sdk::types::crypto::PublicKey;
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::network_name::NetworkName;
use secret_storage::Signer;
use serde::de::DeserializeOwned;

#[cfg(not(target_arch = "wasm32"))]
use iota_interaction::IotaClient;
#[cfg(target_arch = "wasm32")]
use iota_interaction_ts::bindings::WasmIotaClient as IotaClient;

/// A marker type indicating the absence of a signer.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct NoSigner;

/// The error that results from a failed attempt at creating an [IdentityClient]
/// from a given [IotaClient].
#[derive(Debug, thiserror::Error)]
#[error("failed to create an 'IdentityClient' from the given 'IotaClient'")]
#[non_exhaustive]
pub struct FromIotaClientError {
    /// Type of failure for this error.
    #[source]
    pub kind: FromIotaClientErrorKind,
}

/// Types of failure for [FromIotaClientError].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FromIotaClientErrorKind {
    /// A package ID is required, but was not supplied.
    #[error("an IOTA Identity package ID must be supplied when connecting to an unofficial IOTA network")]
    MissingPackageId,
    /// Network ID resolution through an RPC call failed.
    #[error("failed to resolve the network the given client is connected to")]
    NetworkResolution(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// A full client that wraps the read-only client and hosts write operations.
#[derive(Clone)]
pub struct AuditTrailClient<S> {
    pub(super) read_client: AuditTrailClientReadOnly,
    pub(super) public_key: Option<PublicKey>,
    pub(super) signer: S,
}

impl<S> Deref for AuditTrailClient<S> {
    type Target = AuditTrailClientReadOnly;
    fn deref(&self) -> &Self::Target {
        &self.read_client
    }
}

impl AuditTrailClient<NoSigner> {
    /// Creates a new [AuditTrailClient], with **no** signing capabilities, from the given [IotaClient].
    ///
    /// # Warning
    /// Passing a `custom_package_id` is **only** required when connecting to a custom IOTA network.
    ///
    /// Relying on a custom Audit Trail package when connected to an official IOTA network is **highly
    /// discouraged** and is sure to result in compatibility issues when interacting with other official
    /// IOTA Trust Framework's products.
    ///
    /// # Examples
    /// ```
    /// # use audit_trails::client::AuditTrailClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let iota_client = iota_sdk::IotaClientBuilder::default()
    ///   .build_testnet()
    ///   .await?;
    /// // No package ID is required since we are connecting to an official IOTA network.
    /// let audit_trail_client = AuditTrailClient::from_iota_client(iota_client, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_iota_client(
        iota_client: IotaClient,
        custom_package_id: impl Into<Option<ObjectID>>,
    ) -> Result<Self, FromIotaClientError> {
        let read_only_client = if let Some(custom_package_id) = custom_package_id.into() {
        AuditTrailClientReadOnly::new_with_pkg_id(iota_client, custom_package_id).await
    } else {
        AuditTrailClientReadOnly::new(iota_client).await
    }
    .map_err(|e| match e {
        Error::InvalidConfig(_) => FromIotaClientErrorKind::MissingPackageId,
        Error::RpcError(msg) => FromIotaClientErrorKind::NetworkResolution(msg.into()),
        _ => unreachable!("'AuditTrailClientReadOnly::new' has been changed without updating error handling in 'AuditTrailClient::from_iota_client'"),
    })
    .map_err(|kind| FromIotaClientError { kind })?;

        Ok(Self {
            read_client: read_only_client,
            public_key: None,
            signer: NoSigner,
        })
    }
}

impl<S> AuditTrailClient<S> {
    /// Sets a new signer for this client.
    pub async fn with_signer<NewS>(self, signer: NewS) -> Result<AuditTrailClient<NewS>, secret_storage::Error>
    where
        NewS: Signer<IotaKeySignature>,
    {
        let public_key = signer.public_key().await?;

        Ok(AuditTrailClient {
            read_client: self.read_client,
            public_key: Some(public_key),
            signer,
        })
    }
    pub fn read_only(&self) -> &AuditTrailClientReadOnly {
        &self.read_client
    }

    pub fn trail<'a>(&'a self, trail_id: ObjectID) -> AuditTrailHandle<'a, Self> {
        AuditTrailHandle::new(self, trail_id)
    }

    /// Creates a builder for an audit trail.
    pub fn create_trail(&self) -> AuditTrailBuilder {
        AuditTrailBuilder {
            admin: self.public_key.as_ref().map(IotaAddress::from),
            ..AuditTrailBuilder::default()
        }
    }

    pub async fn migrate(&self, _trail_id: ObjectID) -> Result<(), Error> {
        Err(Error::NotImplemented("AuditTrailClient::migrate"))
    }

    pub async fn delete_trail(&self, _trail_id: ObjectID) -> Result<(), Error> {
        Err(Error::NotImplemented("AuditTrailClient::delete_trail"))
    }
}

impl<S> AuditTrailClient<S>
where
    S: Signer<IotaKeySignature>,
{
    /// Returns a reference to the [PublicKey] wrapped by this client.
    pub fn public_key(&self) -> &PublicKey {
        self.public_key.as_ref().expect("public_key is set")
    }

    /// Returns the [IotaAddress] wrapped by this client.
    #[inline(always)]
    pub fn address(&self) -> IotaAddress {
        IotaAddress::from(self.public_key())
    }
}

#[cfg_attr(feature = "send-sync", async_trait)]
#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
impl<S> CoreClientReadOnly for AuditTrailClient<S> {
    fn package_id(&self) -> ObjectID {
        self.read_client.package_id()
    }

    fn network_name(&self) -> &NetworkName {
        self.read_client.network()
    }

    fn client_adapter(&self) -> &IotaClientAdapter {
        &self.read_client
    }
}

#[cfg_attr(feature = "send-sync", async_trait)]
#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
impl<S> CoreClient<S> for AuditTrailClient<S>
where
    S: Signer<IotaKeySignature> + OptionalSync,
{
    fn signer(&self) -> &S {
        &self.signer
    }

    fn sender_address(&self) -> IotaAddress {
        IotaAddress::from(self.public_key())
    }

    fn sender_public_key(&self) -> &PublicKey {
        self.public_key()
    }
}

#[async_trait::async_trait]
impl<S> AuditTrailReadOnly for AuditTrailClient<S>
where
    S: Signer<IotaKeySignature> + OptionalSync,
{
    async fn execute_read_only_transaction<T: DeserializeOwned>(
        &self,
        tx: ProgrammableTransaction,
    ) -> Result<T, Error> {
        self.read_client.execute_read_only_transaction(tx).await
    }
}

impl<S> AuditTrailFull for AuditTrailClient<S> where S: Signer<IotaKeySignature> + OptionalSync {}
