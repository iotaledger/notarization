// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Role and capability management APIs for audit trails.
//!
//! This module is the Rust-facing wrapper around the access-control state integrated into each audit trail.
//! Roles grant [`PermissionSet`] values, while capability objects bind one role to one trail and may add
//! optional address or time restrictions.
//!
//! Additional record-tag constraints are represented as [`RoleTags`]. They narrow which tagged records a role
//! may operate on, but they do not replace the underlying permission checks enforced by the Move package.

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
///
/// This handle exposes role-management and capability-management operations for one trail. All authorization is
/// still enforced against the capability supplied during transaction construction.
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

    /// Returns a role-scoped handle for the given role name.
    ///
    /// The returned handle only identifies the role. Existence and authorization are checked when the
    /// resulting transaction is built and executed.
    pub fn for_role(&self, name: impl Into<String>) -> RoleHandle<'a, C> {
        RoleHandle::new(self.client, self.trail_id, name.into(), self.selected_capability_id)
    }

    /// Revokes an issued capability.
    ///
    /// Revocation adds the capability ID to the trail's denylist. Pass the capability's `valid_until` value
    /// when it is known so later cleanup keeps the same expiry semantics.
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
    ///
    /// This consumes the owned capability object itself. It uses the generic capability-destruction path and
    /// therefore must not be used for initial-admin capabilities.
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

    /// Destroys an initial-admin capability without presenting another authorization capability.
    ///
    /// Initial-admin capability IDs are tracked separately, so they cannot be removed through the generic
    /// destroy path.
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

    /// Revokes an initial-admin capability by ID.
    ///
    /// Like [`TrailAccess::revoke_capability`], this writes to the denylist. The dedicated entry point exists
    /// because initial-admin capability IDs are protected separately.
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
    ///
    /// Only entries whose stored expiry has passed are removed. Revocations without an expiry remain until
    /// they are explicitly destroyed or the trail is deleted.
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

/// Role-scoped access-control API.
///
/// A `RoleHandle` identifies one role name inside the trail's access-control state and builds transactions that
/// act on that role.
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

    /// Returns the role name represented by this handle.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Creates this role with the provided permissions and optional role-tag
    /// access rules.
    ///
    /// Any supplied [`RoleTags`] must already exist in the trail-owned tag
    /// registry. The tag list is stored as
    /// role data on the Move side and is later used for tag-aware record authorization.
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
    ///
    /// The resulting capability always targets this trail and grants exactly
    /// this role. `issued_to`,
    /// `valid_from_ms`, and `valid_until_ms` only configure restrictions on
    /// the issued object; enforcement
    /// happens on-chain when the capability is later used.
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
    ///
    /// As with [`RoleHandle::create`], any supplied [`RoleTags`] must already
    /// exist in the trail tag registry.
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
    ///
    /// The reserved initial-admin role cannot be deleted.
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
