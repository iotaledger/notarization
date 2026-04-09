// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

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

#[derive(Debug, Clone)]
pub struct TrailAccess<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
    pub(crate) selected_capability_id: Option<ObjectID>,
}

impl<'a, C> TrailAccess<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID, selected_capability_id: Option<ObjectID>) -> Self {
        Self {
            client,
            trail_id,
            selected_capability_id,
        }
    }

    /// Uses the provided capability as the auth capability for subsequent write operations.
    pub fn using_capability(mut self, capability_id: ObjectID) -> Self {
        self.selected_capability_id = Some(capability_id);
        self
    }

    /// Returns a handle bound to a specific role name.
    pub fn for_role(&self, name: impl Into<String>) -> RoleHandle<'a, C> {
        RoleHandle::new(self.client, self.trail_id, name.into(), self.selected_capability_id)
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
            self.selected_capability_id,
        ))
    }

    /// Destroys a capability object.
    pub fn destroy_capability<S>(&self, capability_id: ObjectID) -> TransactionBuilder<DestroyCapability>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(DestroyCapability::new(
            self.trail_id,
            owner,
            capability_id,
            self.selected_capability_id,
        ))
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
            self.selected_capability_id,
        ))
    }

    /// Removes expired entries from the revoked-capability denylist.
    pub fn cleanup_revoked_capabilities<S>(&self) -> TransactionBuilder<CleanupRevokedCapabilities>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(CleanupRevokedCapabilities::new(
            self.trail_id,
            owner,
            self.selected_capability_id,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct RoleHandle<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
    pub(crate) name: String,
    pub(crate) selected_capability_id: Option<ObjectID>,
}

impl<'a, C> RoleHandle<'a, C> {
    pub(crate) fn new(
        client: &'a C,
        trail_id: ObjectID,
        name: String,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            client,
            trail_id,
            name,
            selected_capability_id,
        }
    }

    /// Uses the provided capability as the auth capability for subsequent write operations.
    pub fn using_capability(mut self, capability_id: ObjectID) -> Self {
        self.selected_capability_id = Some(capability_id);
        self
    }

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
            self.selected_capability_id,
        ))
    }

    /// Issues a capability for this role using optional restrictions.
    pub fn issue_capability<S>(&self, options: CapabilityIssueOptions) -> TransactionBuilder<IssueCapability>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(IssueCapability::new(
            self.trail_id,
            owner,
            self.name.clone(),
            options,
            self.selected_capability_id,
        ))
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
            self.selected_capability_id,
        ))
    }

    /// Deletes this role.
    pub fn delete<S>(&self) -> TransactionBuilder<DeleteRole>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(DeleteRole::new(
            self.trail_id,
            owner,
            self.name.clone(),
            self.selected_capability_id,
        ))
    }
}
