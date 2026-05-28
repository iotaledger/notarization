// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use audit_trail::AuditTrailClient;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result};
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::{into_transaction_builder, parse_wasm_object_id};
use product_common::bindings::WasmObjectID;
use wasm_bindgen::prelude::*;

use crate::trail::{
    WasmCleanupRevokedCapabilities, WasmCreateRole, WasmDeleteRole, WasmDestroyCapability,
    WasmDestroyInitialAdminCapability, WasmIssueCapability, WasmRevokeCapability, WasmRevokeInitialAdminCapability,
    WasmUpdateRole,
};
use crate::types::{WasmCapabilityIssueOptions, WasmPermissionSet, WasmRoleTags};

/// Access-control API scoped to a specific trail.
///
/// @remarks
/// Exposes role-management and capability-management operations for one trail. Per-role operations
/// live on {@link RoleHandle}, which is reached through {@link TrailAccess.forRole}.
#[derive(Clone)]
#[wasm_bindgen(js_name = TrailAccess, inspectable)]
pub struct WasmTrailAccess {
    pub(crate) full: Option<AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
}

impl WasmTrailAccess {
    fn require_write(&self) -> Result<&AuditTrailClient<WasmTransactionSigner>> {
        self.full.as_ref().ok_or_else(|| {
            wasm_error(anyhow!(
                "TrailAccess was created from a read-only client; this operation requires AuditTrailClient"
            ))
        })
    }
}

#[wasm_bindgen(js_class = TrailAccess)]
impl WasmTrailAccess {
    /// Returns a role-scoped handle for the given role name.
    ///
    /// @remarks
    /// The returned handle only identifies the role. If a role with `name` does not yet exist, the
    /// handle can still be used to create it via {@link RoleHandle.create}.
    ///
    /// @param name - Role name to bind the handle to.
    ///
    /// @returns A {@link RoleHandle} bound to `name` inside this trail.
    #[wasm_bindgen(js_name = forRole)]
    pub fn for_role(&self, name: String) -> WasmRoleHandle {
        WasmRoleHandle {
            full: self.full.clone(),
            trail_id: self.trail_id,
            name,
        }
    }

    /// Builds a capability-revocation transaction.
    ///
    /// @remarks
    /// Adds `capabilityId` to the trail's revoked-capability denylist. Initial-admin capabilities
    /// cannot be revoked through this path — use
    /// {@link TrailAccess.revokeInitialAdminCapability} instead.
    ///
    /// Requires the {@link Permission.RevokeCapabilities} permission.
    ///
    /// @param capabilityId - Object ID of the capability to revoke.
    /// @param capabilityValidUntil - Original capability expiry in milliseconds since the Unix
    /// epoch. Pass it so {@link CleanupRevokedCapabilities} can later prune the denylist entry once
    /// the timestamp has elapsed; pass `null` to keep the entry permanently.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link RevokeCapability} transaction.
    ///
    /// @throws When `capabilityId` is malformed or the wrapper was created from a read-only
    /// client.
    ///
    /// Emits a {@link CapabilityRevoked} event on success.
    #[wasm_bindgen(js_name = revokeCapability, unchecked_return_type = "TransactionBuilder<RevokeCapability>")]
    pub fn revoke_capability(
        &self,
        capability_id: WasmObjectID,
        capability_valid_until: Option<u64>,
    ) -> Result<WasmTransactionBuilder> {
        let capability_id = parse_wasm_object_id(&capability_id)?;
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .revoke_capability(capability_id, capability_valid_until)
            .into_inner();
        Ok(into_transaction_builder(WasmRevokeCapability(tx)))
    }

    /// Builds a capability-destruction transaction.
    ///
    /// @remarks
    /// Consumes the owned capability object and removes any matching denylist entry. This path is
    /// for ordinary capabilities only — initial-admin capabilities must use
    /// {@link TrailAccess.destroyInitialAdminCapability}.
    ///
    /// Requires the {@link Permission.RevokeCapabilities} permission.
    ///
    /// @param capabilityId - Object ID of the capability to destroy.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link DestroyCapability} transaction.
    ///
    /// @throws When `capabilityId` is malformed or the wrapper was created from a read-only
    /// client.
    ///
    /// Emits a {@link CapabilityDestroyed} event on success.
    #[wasm_bindgen(js_name = destroyCapability, unchecked_return_type = "TransactionBuilder<DestroyCapability>")]
    pub fn destroy_capability(&self, capability_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
        let capability_id = parse_wasm_object_id(&capability_id)?;
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .destroy_capability(capability_id)
            .into_inner();
        Ok(into_transaction_builder(WasmDestroyCapability(tx)))
    }

    /// Builds an initial-admin-capability destruction transaction.
    ///
    /// @remarks
    /// Self-service: the holder consumes their own initial-admin capability without presenting
    /// another authorization capability. Initial-admin capability IDs are tracked separately and
    /// cannot be removed through the generic destroy path. **Warning:** if every initial-admin
    /// capability is destroyed (and none was issued separately), the trail is permanently sealed
    /// with no admin access possible.
    ///
    /// @param capabilityId - Object ID of the initial-admin capability to destroy.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link DestroyInitialAdminCapability} transaction.
    ///
    /// @throws When `capabilityId` is malformed or the wrapper was created from a read-only
    /// client.
    ///
    /// Emits a {@link CapabilityDestroyed} event on success.
    #[wasm_bindgen(js_name = destroyInitialAdminCapability, unchecked_return_type = "TransactionBuilder<DestroyInitialAdminCapability>")]
    pub fn destroy_initial_admin_capability(&self, capability_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
        let capability_id = parse_wasm_object_id(&capability_id)?;
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .destroy_initial_admin_capability(capability_id)
            .into_inner();
        Ok(into_transaction_builder(WasmDestroyInitialAdminCapability(tx)))
    }

    /// Builds an initial-admin-capability revocation transaction.
    ///
    /// @remarks
    /// Same denylist semantics as {@link TrailAccess.revokeCapability} but uses the dedicated entry
    /// point reserved for initial-admin capability IDs. **Warning:** revoking every initial-admin
    /// capability permanently seals the trail with no admin access possible.
    ///
    /// Requires the {@link Permission.RevokeCapabilities} permission.
    ///
    /// @param capabilityId - Object ID of the initial-admin capability to revoke.
    /// @param capabilityValidUntil - Original capability expiry in milliseconds since the Unix
    /// epoch. Pass it so {@link CleanupRevokedCapabilities} can later prune the denylist entry once
    /// the timestamp has elapsed; pass `null` to keep the entry permanently.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link RevokeInitialAdminCapability} transaction.
    ///
    /// @throws When `capabilityId` is malformed or the wrapper was created from a read-only
    /// client.
    ///
    /// Emits a {@link CapabilityRevoked} event on success.
    #[wasm_bindgen(js_name = revokeInitialAdminCapability, unchecked_return_type = "TransactionBuilder<RevokeInitialAdminCapability>")]
    pub fn revoke_initial_admin_capability(
        &self,
        capability_id: WasmObjectID,
        capability_valid_until: Option<u64>,
    ) -> Result<WasmTransactionBuilder> {
        let capability_id = parse_wasm_object_id(&capability_id)?;
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .revoke_initial_admin_capability(capability_id, capability_valid_until)
            .into_inner();
        Ok(into_transaction_builder(WasmRevokeInitialAdminCapability(tx)))
    }

    /// Builds a cleanup transaction for expired revoked-capability entries.
    ///
    /// @remarks
    /// Only prunes denylist entries whose stored `validUntil` is non-zero and strictly less than
    /// the current clock time. Entries with `validUntil == 0` (revocations without a known expiry)
    /// remain on the denylist indefinitely. Does not revoke additional capabilities and does not
    /// destroy any objects.
    ///
    /// Requires the {@link Permission.RevokeCapabilities} permission.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link CleanupRevokedCapabilities} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
    ///
    /// Emits a {@link RevokedCapabilitiesCleanedUp} event on success.
    #[wasm_bindgen(js_name = cleanupRevokedCapabilities, unchecked_return_type = "TransactionBuilder<CleanupRevokedCapabilities>")]
    pub fn cleanup_revoked_capabilities(&self) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .cleanup_revoked_capabilities()
            .into_inner();
        Ok(into_transaction_builder(WasmCleanupRevokedCapabilities(tx)))
    }
}

/// Role-scoped access-control API.
///
/// @remarks
/// Identifies one role name inside the trail's access-control state and builds transactions that
/// act on that role.
#[derive(Clone)]
#[wasm_bindgen(js_name = RoleHandle, inspectable)]
pub struct WasmRoleHandle {
    pub(crate) full: Option<AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
    pub(crate) name: String,
}

impl WasmRoleHandle {
    fn require_write(&self) -> Result<&AuditTrailClient<WasmTransactionSigner>> {
        self.full.as_ref().ok_or_else(|| {
            wasm_error(anyhow!(
                "RoleHandle was created from a read-only client; this operation requires AuditTrailClient"
            ))
        })
    }
}

#[wasm_bindgen(js_class = RoleHandle)]
impl WasmRoleHandle {
    /// Returns the role name represented by this handle.
    ///
    /// @returns The role name bound to this handle.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Builds a role-creation transaction.
    ///
    /// @remarks
    /// Creates this role with `permissions` and the optional `roleTags` allowlist. Each tag
    /// referenced by `roleTags` must already exist in the trail-owned tag registry; the on-chain
    /// call aborts otherwise and bumps that tag's usage counter on success.
    ///
    /// Requires the {@link Permission.AddRoles} permission.
    ///
    /// @param permissions - {@link PermissionSet} granted by the new role.
    /// @param roleTags - Optional {@link RoleTags} allowlist that restricts the role's reach to
    /// records carrying one of these tags.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link CreateRole} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
    ///
    /// Emits a {@link RoleCreated} event on success.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<CreateRole>")]
    pub fn create(
        &self,
        permissions: WasmPermissionSet,
        role_tags: Option<WasmRoleTags>,
    ) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .for_role(self.name.clone())
            .create(permissions.into(), role_tags.map(Into::into))
            .into_inner();
        Ok(into_transaction_builder(WasmCreateRole(tx)))
    }

    /// Builds a capability-issuance transaction for this role.
    ///
    /// @remarks
    /// The resulting capability always targets this trail and grants exactly this role. Only
    /// `options.issuedTo`, `options.validFromMs`, and `options.validUntilMs` configure restrictions
    /// on the issued object; enforcement happens on-chain when the capability is later presented
    /// for authorization. The capability is transferred to `options.issuedTo` if set, otherwise to
    /// the caller.
    ///
    /// Requires the {@link Permission.AddCapabilities} permission.
    ///
    /// @param options - {@link CapabilityIssueOptions} configuring recipient and validity window.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link IssueCapability} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
    ///
    /// Emits a {@link CapabilityIssued} event on success.
    #[wasm_bindgen(js_name = issueCapability, unchecked_return_type = "TransactionBuilder<IssueCapability>")]
    pub fn issue_capability(&self, options: WasmCapabilityIssueOptions) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .for_role(self.name.clone())
            .issue_capability(options.into())
            .into_inner();
        Ok(into_transaction_builder(WasmIssueCapability(tx)))
    }

    /// Builds a role-update transaction for this role.
    ///
    /// @remarks
    /// Replaces both the role's permission set and its `roleTags` allowlist. Any newly supplied tag
    /// must already exist in the trail's record-tag registry; tag usage counters are adjusted to
    /// reflect the difference between the old and the new role-tag sets. Updating the
    /// initial-admin role with permissions that do not include every permission configured in the
    /// trail's role- and capability-admin permission sets aborts on-chain.
    ///
    /// Requires the {@link Permission.UpdateRoles} permission.
    ///
    /// @param permissions - Replacement {@link PermissionSet} for the role.
    /// @param roleTags - Replacement {@link RoleTags} allowlist, or `null` to clear the
    /// restriction.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link UpdateRole} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
    ///
    /// Emits a {@link RoleUpdated} event on success.
    #[wasm_bindgen(js_name = updatePermissions, unchecked_return_type = "TransactionBuilder<UpdateRole>")]
    pub fn update_permissions(
        &self,
        permissions: WasmPermissionSet,
        role_tags: Option<WasmRoleTags>,
    ) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .for_role(self.name.clone())
            .update_permissions(permissions.into(), role_tags.map(Into::into))
            .into_inner();
        Ok(into_transaction_builder(WasmUpdateRole(tx)))
    }

    /// Builds a role-deletion transaction for this role.
    ///
    /// @remarks
    /// Decrements the usage count of every tag the role's `roleTags` referenced. The reserved
    /// initial-admin role cannot be deleted.
    ///
    /// Requires the {@link Permission.DeleteRoles} permission.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link DeleteRole} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
    ///
    /// Emits a {@link RoleDeleted} event on success.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<DeleteRole>")]
    pub fn delete(&self) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .for_role(self.name.clone())
            .delete()
            .into_inner();
        Ok(into_transaction_builder(WasmDeleteRole(tx)))
    }
}
