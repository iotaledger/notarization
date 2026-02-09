// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::ObjectID;

use super::{AuditTrailFull, AuditTrailReadOnly};
use crate::core::types::{LockingConfig, LockingWindow};
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct TrailLocking<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
}

impl<'a, C> TrailLocking<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self { client, trail_id }
    }

    pub async fn update(&self, _config: LockingConfig) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailLocking::update"))
    }

    pub async fn update_delete_record_window(&self, _window: LockingWindow) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailLocking::update_delete_record_window"))
    }

    pub async fn is_record_locked(&self, _sequence_number: u64) -> Result<bool, Error>
    where
        C: AuditTrailReadOnly,
    {
        Err(Error::NotImplemented("TrailLocking::is_record_locked"))
    }
}
