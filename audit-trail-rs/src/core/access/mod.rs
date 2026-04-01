// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Role and capability management APIs for audit trails.

use iota_interaction::types::base_types::ObjectID;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::CoreClient;
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;

use crate::core::trail::AuditTrailFull;
use crate::core::types::{CapabilityIssueOptions, PermissionSet, RoleTags};

mod operations;
mod transactions;

pub use transactions::{
    CleanupRevokedCapabilities, CreateRole, DeleteRole, DestroyCapability, DestroyInitialAdminCapability,
    IssueCapability, RevokeCapability, RevokeInitialAdminCapability, UpdateRole,
};

/// Access-control API scoped to a specific trail.
#[derive(Debug, Clone)]
pub struct TrailAccess<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
}

impl<'a, C> TrailAccess<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self { client, trail_id }
    }

    /// Returns a role-scoped handle for the given role name.
    pub fn for_role(&self, name: impl Into<String>) -> RoleHandle<'a, C> {
        RoleHandle::new(self.client, self.trail_id, name.into())
    }

    /// Revokes an issued capability.
    ///
    /// Pass the capability's `valid_until` value when it is known so the denylist entry matches the on-chain cleanup
    /// model.
    pub fn revoke_capability<S>(
        &self,
        capability_id: ObjectID,
        capability_valid_until: Option<u64>,
    ) -> TransactionBuilder<RevokeCapability>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(RevokeCapability::new(
            self.trail_id,
            owner,
            capability_id,
            capability_valid_until,
        ))
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
    ///
    /// Pass the capability's `valid_until` value when it is known so the denylist entry matches the on-chain cleanup
    /// model.
    pub fn revoke_initial_admin_capability<S>(
        &self,
        capability_id: ObjectID,
        capability_valid_until: Option<u64>,
    ) -> TransactionBuilder<RevokeInitialAdminCapability>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(RevokeInitialAdminCapability::new(
            self.trail_id,
            owner,
            capability_id,
            capability_valid_until,
        ))
    }

    /// Removes expired entries from the revoked-capability denylist.
    pub fn cleanup_revoked_capabilities<S>(&self) -> TransactionBuilder<CleanupRevokedCapabilities>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(CleanupRevokedCapabilities::new(self.trail_id, owner))
    }
}

/// Role-scoped access-control API.
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

    /// Returns the role name represented by this handle.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Creates this role with the provided permissions and optional role-tag access rules.
    pub fn create<S>(&self, permissions: PermissionSet, role_tags: Option<RoleTags>) -> TransactionBuilder<CreateRole>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(CreateRole::new(
            self.trail_id,
            owner,
            self.name.clone(),
            permissions,
            role_tags,
        ))
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

    /// Updates permissions and role-tag access rules for this role.
    pub fn update_permissions<S>(
        &self,
        permissions: PermissionSet,
        role_tags: Option<RoleTags>,
    ) -> TransactionBuilder<UpdateRole>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(UpdateRole::new(
            self.trail_id,
            owner,
            self.name.clone(),
            permissions,
            role_tags,
        ))
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
