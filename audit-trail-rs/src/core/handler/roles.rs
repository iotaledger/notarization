// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::ObjectID;

use super::AuditTrailFull;
use crate::core::types::PermissionSet;
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct TrailRoles<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
}

impl<'a, C> TrailRoles<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self { client, trail_id }
    }

    /// Returns a handle bound to a specific role name.
    pub fn role(&self, name: impl Into<String>) -> RoleHandle<'a, C> {
        RoleHandle::new(self.client, self.trail_id, name.into())
    }

    /// Creates a new role with the provided permissions.
    pub async fn create(&self, _name: impl Into<String>, _permissions: PermissionSet) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailRoles::create"))
    }

    /// Updates permissions for an existing role.
    pub async fn update(&self, _name: impl Into<String>, _permissions: PermissionSet) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailRoles::update"))
    }

    /// Deletes an existing role.
    pub async fn delete(&self, _name: impl Into<String>) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailRoles::delete"))
    }
}

#[derive(Debug, Clone)]
pub struct RoleHandle<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
    pub(crate) name: String,
}

impl<'a, C> RoleHandle<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID, name: String) -> Self {
        Self { client, trail_id, name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Updates permissions for this role.
    pub async fn update_permissions(&self, _permissions: PermissionSet) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("RoleHandle::update_permissions"))
    }

    /// Deletes this role.
    pub async fn delete(&self) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("RoleHandle::delete"))
    }
}
