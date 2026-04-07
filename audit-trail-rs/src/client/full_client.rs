// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Audit Trail Client
//!
//! The full client extends [`AuditTrailClientReadOnly`] with signing support and write
//! transaction builders.
//!
//! ## Transaction Flow
//!
//! Write APIs return a [`TransactionBuilder`](product_common::transaction::transaction_builder::TransactionBuilder)
//! that you can configure before signing and submitting:
//!
//! ```rust,no_run
//! # use audit_trail::AuditTrailClient;
//! # use audit_trail::core::types::Data;
//! # async fn example(
//! #     client: &AuditTrailClient<
//! #         impl secret_storage::Signer<iota_interaction::IotaKeySignature> + iota_interaction::OptionalSync,
//! #     >,
//! # ) -> Result<(), Box<dyn std::error::Error>> {
//! let created = client
//!     .create_trail()
//!     .with_initial_record_parts(Data::text("Initial record"), None, None)
//!     .finish()
//!     .with_gas_budget(1_000_000)
//!     .build_and_execute(client)
//!     .await?;
//!
//! let trail_id = created.output.trail_id;
//!
//! client
//!     .trail(trail_id)
//!     .records()
//!     .add(Data::text("Follow-up record"), None, None)
//!     .build_and_execute(client)
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Example Workflow
//!
//! ```rust,no_run
//! # use audit_trail::AuditTrailClient;
//! # use audit_trail::core::types::{Data, PermissionSet, RoleTags};
//! # async fn example(
//! #     client: &AuditTrailClient<
//! #         impl secret_storage::Signer<iota_interaction::IotaKeySignature> + iota_interaction::OptionalSync,
//! #     >,
//! # ) -> Result<(), Box<dyn std::error::Error>> {
//! let created = client
//!     .create_trail()
//!     .with_initial_record_parts(Data::text("Initial record"), None, None)
//!     .with_record_tags(["finance"])
//!     .finish()
//!     .build_and_execute(client)
//!     .await?;
//!
//! let trail_id = created.output.trail_id;
//!
//! client
//!     .trail(trail_id)
//!     .access()
//!     .for_role("TaggedWriter")
//!     .create(PermissionSet::record_admin_permissions(), Some(RoleTags::new(["finance"])))
//!     .build_and_execute(client)
//!     .await?;
//!
//! client
//!     .trail(trail_id)
//!     .records()
//!     .add(Data::text("Budget approved"), None, Some("finance".to_string()))
//!     .build_and_execute(client)
//!     .await?;
//! # Ok(())
//! # }
//! ```

use std::ops::Deref;

use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use iota_interaction::IotaClient;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::crypto::PublicKey;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaKeySignature, OptionalSync};
#[cfg(target_arch = "wasm32")]
use iota_interaction_ts::bindings::WasmIotaClient as IotaClient;
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::network_name::NetworkName;
use secret_storage::Signer;
use serde::de::DeserializeOwned;

use crate::client::read_only::{AuditTrailClientReadOnly, PackageOverrides};
use crate::core::builder::AuditTrailBuilder;
use crate::core::trail::{AuditTrailFull, AuditTrailHandle, AuditTrailReadOnly};
use crate::error::Error;
use crate::iota_interaction_adapter::IotaClientAdapter;

/// A marker type indicating the absence of a signer.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct NoSigner;

/// Error returned when constructing an [`AuditTrailClient`] from an IOTA client fails.
#[derive(Debug, thiserror::Error)]
#[error("failed to create an 'AuditTrailClient' from the given 'IotaClient'")]
#[non_exhaustive]
pub struct FromIotaClientError {
    /// Type of failure for this error.
    #[source]
    pub kind: FromIotaClientErrorKind,
}

/// Categories of failure for [`FromIotaClientError`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FromIotaClientErrorKind {
    /// A package ID is required, but was not supplied.
    #[error("an audit-trail package ID must be supplied when connecting to an unofficial IOTA network")]
    MissingPackageId,
    /// Network ID resolution through an RPC call failed.
    #[error("failed to resolve the network the given client is connected to")]
    NetworkResolution(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// A client for creating and managing audit trails on the IOTA blockchain.
///
/// This client combines read-only capabilities with transaction signing,
/// enabling full interaction with audit trails.
///
/// ## Type Parameter
///
/// - `S`: The signer type that implements [`Signer<IotaKeySignature>`]
#[derive(Clone)]
pub struct AuditTrailClient<S> {
    /// The underlying read-only client used for executing read-only operations.
    pub(super) read_client: AuditTrailClientReadOnly,
    /// The public key associated with the signer, if any.
    pub(super) public_key: Option<PublicKey>,
    /// The signer used for signing transactions, or `NoSigner` if the client is read-only.
    pub(super) signer: S,
}

impl<S> Deref for AuditTrailClient<S> {
    type Target = AuditTrailClientReadOnly;
    fn deref(&self) -> &Self::Target {
        &self.read_client
    }
}

impl AuditTrailClient<NoSigner> {
    /// Creates a new client with no signing capabilities from an IOTA client.
    ///
    /// # Warning
    ///
    /// Passing `package_overrides` is only needed when connecting to a custom IOTA network or
    /// when testing against explicitly deployed package pairs.
    ///
    /// Relying on a custom audit-trail package while connected to an official IOTA network is
    /// strongly discouraged and can lead to compatibility problems with other official IOTA Trust
    /// Framework products.
    ///
    /// # Examples
    /// ```rust,ignore
    /// # use audit_trail::client::AuditTrailClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> anyhow::Result<()> {
    /// let iota_client = iota_sdk::IotaClientBuilder::default()
    ///     .build_testnet()
    ///     .await?;
    /// // No package ID is required since we are connecting to an official IOTA network.
    /// let audit_trail_client = AuditTrailClient::from_iota_client(iota_client, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_iota_client(
        iota_client: IotaClient,
        package_overrides: impl Into<Option<PackageOverrides>>,
    ) -> Result<Self, FromIotaClientError> {
        let read_only_client = if let Some(package_overrides) = package_overrides.into() {
            AuditTrailClientReadOnly::new_with_package_overrides(iota_client, package_overrides).await
        } else {
            AuditTrailClientReadOnly::new(iota_client).await
        }
        .map_err(|e| match e {
            Error::InvalidConfig(_) => FromIotaClientErrorKind::MissingPackageId,
            Error::RpcError(msg) => FromIotaClientErrorKind::NetworkResolution(msg.into()),
            _ => unreachable!(
                "'AuditTrailClientReadOnly::new' has been changed without updating error handling in 'AuditTrailClient::from_iota_client'"
            ),
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
    /// Creates a signing client from an existing read-only client and signer.
    ///
    /// # Errors
    ///
    /// Returns an error if the signer public key cannot be loaded.
    pub async fn new(client: AuditTrailClientReadOnly, signer: S) -> Result<Self, Error>
    where
        S: Signer<IotaKeySignature>,
    {
        let public_key = signer
            .public_key()
            .await
            .map_err(|e| Error::InvalidKey(e.to_string()))?;

        Ok(AuditTrailClient {
            read_client: client,
            public_key: Some(public_key),
            signer,
        })
    }

    /// Replaces the signer used by this client.
    ///
    /// # Errors
    ///
    /// Returns an error if the replacement signer public key cannot be loaded.
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
    /// Returns the underlying read-only client view.
    pub fn read_only(&self) -> &AuditTrailClientReadOnly {
        &self.read_client
    }

    /// Returns a typed handle bound to a specific trail object ID.
    pub fn trail<'a>(&'a self, trail_id: ObjectID) -> AuditTrailHandle<'a, Self> {
        AuditTrailHandle::new(self, trail_id)
    }

    /// Returns the TfComponents package ID used by this client.
    pub fn tf_components_package_id(&self) -> ObjectID {
        self.read_client.tf_components_package_id()
    }

    /// Creates a builder for a new audit trail.
    ///
    /// When the client has a signer, the builder is pre-populated with that signer's address as
    /// the initial admin.
    pub fn create_trail(&self) -> AuditTrailBuilder {
        AuditTrailBuilder {
            admin: self.public_key.as_ref().map(IotaAddress::from),
            ..AuditTrailBuilder::default()
        }
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

    fn tf_components_package_id(&self) -> Option<ObjectID> {
        Some(self.read_client.tf_components_package_id())
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

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl<S> AuditTrailReadOnly for AuditTrailClient<S>
where
    S: Signer<IotaKeySignature> + OptionalSync,
{
    /// Delegates read-only execution to the wrapped [`AuditTrailClientReadOnly`].
    async fn execute_read_only_transaction<T: DeserializeOwned>(
        &self,
        tx: ProgrammableTransaction,
    ) -> Result<T, Error> {
        self.read_client.execute_read_only_transaction(tx).await
    }
}

impl<S> AuditTrailFull for AuditTrailClient<S> where S: Signer<IotaKeySignature> + OptionalSync {}
