// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use serde::de::DeserializeOwned;

use crate::core::locking::TrailLocking;
use crate::core::metadata::TrailMetadata;
use crate::core::records::TrailRecords;
use crate::core::roles::TrailRoles;
use crate::core::types::Data;
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

/// A typed handle bound to a specific audit trail and client.
#[derive(Debug, Clone)]
pub struct AuditTrailHandle<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
}

impl<'a, C> AuditTrailHandle<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self { client, trail_id }
    }

    pub fn records(&self) -> TrailRecords<'a, C, Data> {
        TrailRecords::new(self.client, self.trail_id)
    }

    pub fn locking(&self) -> TrailLocking<'a, C> {
        TrailLocking::new(self.client, self.trail_id)
    }

    pub fn metadata(&self) -> TrailMetadata<'a, C> {
        TrailMetadata::new(self.client, self.trail_id)
    }

    pub fn roles(&self) -> TrailRoles<'a, C> {
        TrailRoles::new(self.client, self.trail_id)
    }
}
