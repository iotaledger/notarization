// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::ObjectID;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::CoreClient;
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;

use crate::core::trail::AuditTrailFull;
use crate::core::types::{CapabilityIssueOptions, PermissionSet};

mod operations;
mod transactions;

pub use transactions::{
    CreateRole, DeleteRole, DestroyCapability, DestroyInitialAdminCapability, IssueCapability, RevokeCapability,
    RevokeInitialAdminCapability, UpdateRole,
};

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
    pub fn for_role(&self, name: impl Into<String>) -> RoleHandle<'a, C> {
        RoleHandle::new(self.client, self.trail_id, name.into())
    }

    /// Revokes an issued capability.
    pub fn revoke_capability<S>(&self, capability_id: ObjectID) -> TransactionBuilder<RevokeCapability>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(RevokeCapability::new(self.trail_id, owner, capability_id))
    }

    /// Destroys a capability object.
    pub fn destroy_capability<S>(&self, capability_id: ObjectID) -> TransactionBuilder<DestroyCapability>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(DestroyCapability::new(self.trail_id, owner, capability_id))
    }

    /// Destroys an initial admin capability (self-service, no auth cap required).
    pub fn destroy_initial_admin_capability<S>(
        &self,
        capability_id: ObjectID,
    ) -> TransactionBuilder<DestroyInitialAdminCapability>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        TransactionBuilder::new(DestroyInitialAdminCapability::new(self.trail_id, capability_id))
    }

    /// Revokes an initial admin capability by ID.
    pub fn revoke_initial_admin_capability<S>(
        &self,
        capability_id: ObjectID,
    ) -> TransactionBuilder<RevokeInitialAdminCapability>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(RevokeInitialAdminCapability::new(self.trail_id, owner, capability_id))
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

    /// Creates this role with the provided permissions.
    pub fn create<S>(&self, permissions: PermissionSet) -> TransactionBuilder<CreateRole>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(CreateRole::new(self.trail_id, owner, self.name.clone(), permissions))
    }

    /// Issues a capability for this role using optional restrictions.
    pub fn issue_capability<S>(&self, options: CapabilityIssueOptions) -> TransactionBuilder<IssueCapability>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(IssueCapability::new(self.trail_id, owner, self.name.clone(), options))
    }

    /// Updates permissions for this role.
    pub fn update_permissions<S>(&self, permissions: PermissionSet) -> TransactionBuilder<UpdateRole>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(UpdateRole::new(self.trail_id, owner, self.name.clone(), permissions))
    }

    /// Deletes this role.
    pub fn delete<S>(&self) -> TransactionBuilder<DeleteRole>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(DeleteRole::new(self.trail_id, owner, self.name.clone()))
    }
}
