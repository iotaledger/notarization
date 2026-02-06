// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod capabilities;
pub mod handle;
pub mod locking;
pub mod metadata;
pub mod records;
pub mod roles;

pub use capabilities::TrailCapabilities;
pub use handle::AuditTrailHandle;
use iota_interaction::OptionalSync;
use iota_interaction::types::transaction::ProgrammableTransaction;
pub use locking::TrailLocking;
pub use metadata::TrailMetadata;
use product_common::core_client::CoreClientReadOnly;
pub use records::TrailRecords;
pub use roles::TrailRoles;
use serde::de::DeserializeOwned;

use crate::error::Error;

/// Marker trait for read-only audit trail clients.
#[doc(hidden)]
#[async_trait::async_trait]
pub trait AuditTrailReadOnly: CoreClientReadOnly + OptionalSync {
    async fn execute_read_only_transaction<T: DeserializeOwned>(&self, tx: ProgrammableTransaction)
    -> Result<T, Error>;
}

/// Marker trait for full (read-write) audit trail clients.
#[doc(hidden)]
pub trait AuditTrailFull: AuditTrailReadOnly {}
