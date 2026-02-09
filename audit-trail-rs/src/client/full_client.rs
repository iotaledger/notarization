// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! A full client wrapper for audit trail interactions.
//!
//! This client includes signing capabilities for executing transactions.

use std::ops::Deref;

use crate::client::read_only::AuditTrailClientReadOnly;
use crate::core::builder::AuditTrailBuilder;
use crate::core::handler::{AuditTrailFull, AuditTrailHandle, AuditTrailReadOnly};
use crate::error::Error;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{IotaKeySignature, OptionalSync};
use iota_sdk::types::crypto::PublicKey;
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::network_name::NetworkName;
use secret_storage::Signer;
use serde::de::DeserializeOwned;

/// A full client that wraps the read-only client and hosts write operations.
#[derive(Clone)]
pub struct AuditTrailClient<S> {
    read_client: AuditTrailClientReadOnly,
    public_key: PublicKey,
    signer: S,
}

impl<S> Deref for AuditTrailClient<S> {
    type Target = AuditTrailClientReadOnly;
    fn deref(&self) -> &Self::Target {
        &self.read_client
    }
}

impl<S> AuditTrailClient<S>
where
    S: Signer<IotaKeySignature>,
{
    pub async fn new(client: AuditTrailClientReadOnly, signer: S) -> Result<Self, Error> {
        let public_key = signer
            .public_key()
            .await
            .map_err(|e| Error::InvalidKey(e.to_string()))?;

        Ok(Self {
            public_key,
            read_client: client,
            signer,
        })
    }
}

impl<S> AuditTrailClient<S> {
    pub fn read_only(&self) -> &AuditTrailClientReadOnly {
        &self.read_client
    }

    pub fn trail<'a>(&'a self, trail_id: ObjectID) -> AuditTrailHandle<'a, Self> {
        AuditTrailHandle::new(self, trail_id)
    }

    /// Creates a builder for an audit trail.
    pub fn create_trail(&self) -> AuditTrailBuilder {
        AuditTrailBuilder::new()
    }

    pub async fn migrate(&self, _trail_id: ObjectID) -> Result<(), Error> {
        Err(Error::NotImplemented("AuditTrailClient::migrate"))
    }

    pub async fn delete_trail(&self, _trail_id: ObjectID) -> Result<(), Error> {
        Err(Error::NotImplemented("AuditTrailClient::delete_trail"))
    }
}

#[async_trait::async_trait]
impl<S> CoreClientReadOnly for AuditTrailClient<S> {
    fn package_id(&self) -> ObjectID {
        self.read_client.package_id()
    }

    fn network_name(&self) -> &NetworkName {
        self.read_client.network()
    }

    fn client_adapter(&self) -> &crate::iota_interaction_adapter::IotaClientAdapter {
        self.read_client.iota_client()
    }
}

#[async_trait::async_trait]
impl<S> CoreClient<S> for AuditTrailClient<S>
where
    S: Signer<IotaKeySignature> + OptionalSync,
{
    fn signer(&self) -> &S {
        &self.signer
    }

    fn sender_address(&self) -> iota_interaction::types::base_types::IotaAddress {
        iota_interaction::types::base_types::IotaAddress::from(&self.public_key)
    }

    fn sender_public_key(&self) -> &iota_interaction::types::crypto::PublicKey {
        &self.public_key
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
