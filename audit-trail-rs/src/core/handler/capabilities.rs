// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::ObjectID;

use super::AuditTrailFull;
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct TrailCapabilities<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
}

impl<'a, C> TrailCapabilities<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self { client, trail_id }
    }

    pub async fn issue(&self, _role: String) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailCapabilities::issue"))
    }

    pub async fn revoke(&self, _capability_id: ObjectID) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailCapabilities::revoke"))
    }

    pub async fn destroy(&self, _capability_id: ObjectID) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailCapabilities::destroy"))
    }
}
