// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Typed handle wrappers bound to a specific trail id and client reference.

use iota_interaction::types::base_types::ObjectID;

use super::capabilities::TrailCapabilities;
use super::locking::TrailLocking;
use super::metadata::TrailMetadata;
use super::records::TrailRecords;
use super::roles::TrailRoles;
use crate::core::types::Data;

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

    pub fn records_as<D>(&self) -> TrailRecords<'a, C, D> {
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

    pub fn capabilities(&self) -> TrailCapabilities<'a, C> {
        TrailCapabilities::new(self.client, self.trail_id)
    }
}
