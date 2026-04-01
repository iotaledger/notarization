// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Audit trail builder for creation transactions.

use std::collections::HashSet;

use iota_interaction::types::base_types::IotaAddress;
use product_common::transaction::transaction_builder::TransactionBuilder;

use super::types::{Data, ImmutableMetadata, InitialRecord, LockingConfig};
use crate::core::create::CreateTrail;

/// Builder for creating an audit trail.
#[derive(Debug, Clone, Default)]
pub struct AuditTrailBuilder {
    /// Initial admin address that should receive the initial admin capability.
    pub admin: Option<IotaAddress>,
    /// Optional initial record created together with the trail.
    pub initial_record: Option<InitialRecord>,
    /// Locking rules to apply at creation time.
    pub locking_config: LockingConfig,
    /// Immutable metadata stored once at creation time.
    pub trail_metadata: Option<ImmutableMetadata>,
    /// Mutable metadata stored on the trail object.
    pub updatable_metadata: Option<String>,
    /// Canonical list of record tags owned by the trail.
    pub record_tags: HashSet<String>,
}

impl AuditTrailBuilder {
    /// Sets the full initial record input used during trail creation.
    pub fn with_initial_record(mut self, initial_record: InitialRecord) -> Self {
        self.initial_record = Some(initial_record);
        self
    }

    /// Convenience helper for constructing the initial record inline.
    pub fn with_initial_record_parts(
        mut self,
        data: impl Into<Data>,
        metadata: Option<String>,
        tag: Option<String>,
    ) -> Self {
        self.initial_record = Some(InitialRecord::new(data, metadata, tag));
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

    /// Sets the canonical list of tags that may be used on records in this trail.
    pub fn with_record_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.record_tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the admin address that receives the initial admin capability.
    pub fn with_admin(mut self, admin: IotaAddress) -> Self {
        self.admin = Some(admin);
        self
    }

    /// Finalizes the builder and creates a transaction builder.
    pub fn finish(self) -> TransactionBuilder<CreateTrail> {
        TransactionBuilder::new(CreateTrail::new(self))
    }
}
