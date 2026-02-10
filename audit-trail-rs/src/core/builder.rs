// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Audit trail builder for creation transactions.

use iota_sdk::types::base_types::IotaAddress;
use product_common::transaction::transaction_builder::TransactionBuilder;

use super::types::{Data, ImmutableMetadata, LockingConfig};
use crate::core::create::CreateTrail;
use crate::error::Error;

/// Builder for creating an audit trail.
#[derive(Debug, Clone, Default)]
pub struct AuditTrailBuilder {
    pub admin: Option<IotaAddress>,
    pub initial_data: Option<Data>,
    pub initial_record_metadata: Option<String>,
    pub locking_config: LockingConfig,
    pub trail_metadata: Option<ImmutableMetadata>,
    pub updatable_metadata: Option<String>,
}

impl AuditTrailBuilder {
    /// Sets the initial record data and optional record metadata.
    pub fn with_initial_record(mut self, data: Data, metadata: Option<String>) -> Self {
        self.initial_data = Some(data);
        self.initial_record_metadata = metadata;
        self
    }

    /// Sets the locking configuration for the trail.
    pub fn with_locking_config(mut self, config: LockingConfig) -> Self {
        self.locking_config = config;
        self
    }

    /// Sets immutable metadata for the trail.
    pub fn with_trail_metadata(mut self, metadata: ImmutableMetadata) -> Self {
        self.trail_metadata = Some(metadata);
        self
    }

    /// Sets immutable metadata by parts.
    pub fn with_trail_metadata_parts(mut self, name: impl Into<String>, description: Option<String>) -> Self {
        self.trail_metadata = Some(ImmutableMetadata {
            name: name.into(),
            description,
        });
        self
    }

    /// Sets updatable metadata for the trail.
    pub fn with_updatable_metadata(mut self, metadata: impl Into<String>) -> Self {
        self.updatable_metadata = Some(metadata.into());
        self
    }

    /// Sets the admin address that receives the initial admin capability.
    pub fn with_admin(mut self, admin: IotaAddress) -> Self {
        self.admin = Some(admin);
        self
    }

    /// Finalizes the builder and creates a transaction builder.
    pub fn finish(self) -> Result<TransactionBuilder<CreateTrail>, Error> {
        Ok(TransactionBuilder::new(CreateTrail::new(self)))
    }
}
