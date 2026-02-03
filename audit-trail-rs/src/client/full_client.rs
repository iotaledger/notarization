// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! A minimal full client wrapper for audit trail interactions.
//!
//! This is a scaffold that will be extended with transaction-building capabilities
//! once the Move contract API is finalized.

use std::ops::Deref;

use crate::client::read_only::AuditTrailClientReadOnly;

/// A full client that wraps the read-only client and will host write operations.
#[derive(Clone)]
pub struct AuditTrailClient {
    read_client: AuditTrailClientReadOnly,
}

impl Deref for AuditTrailClient {
    type Target = AuditTrailClientReadOnly;
    fn deref(&self) -> &Self::Target {
        &self.read_client
    }
}

impl AuditTrailClient {
    /// Creates a new full client from an existing read-only client.
    pub fn new(read_client: AuditTrailClientReadOnly) -> Self {
        Self { read_client }
    }

    /// Returns a reference to the underlying read-only client.
    pub const fn read_only(&self) -> &AuditTrailClientReadOnly {
        &self.read_client
    }
}
