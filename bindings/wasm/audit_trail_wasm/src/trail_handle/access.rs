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
/// Exposes role-management and capability-management operations for one trail.
#[derive(Clone)]
#[wasm_bindgen(js_name = TrailAccess, inspectable)]
pub struct WasmTrailAccess {
    pub(crate) full: Option<AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
}

impl WasmTrailAccess {
    /// Returns the writable client for access-control operations.
    ///
    /// Throws when this wrapper was created from `AuditTrailClientReadOnly`.
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
    /// The returned handle only identifies the role. If the identified doesn't exist
    /// the specified `name` can be used to create a role.
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
    /// Adds `capabilityId` to the trail's revoked-capability denylist. Pass
    /// `capabilityValidUntil` (the capability's original expiry, in milliseconds since the Unix
    /// epoch) so [`WasmCleanupRevokedCapabilities`](crate::trail::WasmCleanupRevokedCapabilities)
    /// can later prune the entry once that timestamp has elapsed; pass `null` to keep the
    /// denylist entry permanently. Initial-admin capabilities cannot be revoked through this path
    /// — use [`revokeInitialAdminCapability`](Self::revoke_initial_admin_capability) instead.
    /// Requires the `RevokeCapabilities` permission. Emits a `CapabilityRevoked` event on
    /// success.
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
    /// Consumes the owned capability object and removes any matching denylist entry. This path is
    /// for ordinary capabilities only — initial-admin capabilities must use
    /// [`destroyInitialAdminCapability`](Self::destroy_initial_admin_capability). Requires the
    /// `RevokeCapabilities` permission. Emits a `CapabilityDestroyed` event on success.
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
    /// Self-service: the holder consumes their own initial-admin capability without presenting
    /// another authorization capability. Initial-admin capability IDs are tracked separately and
    /// cannot be removed through the generic destroy path. **Warning:** if every initial-admin
    /// capability is destroyed (and none was issued separately), the trail is permanently sealed
    /// with no admin access possible. Emits a `CapabilityDestroyed` event on success.
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
    /// Same denylist semantics as [`revokeCapability`](Self::revoke_capability) but uses the
    /// dedicated entry point reserved for initial-admin capability IDs. **Warning:** revoking
    /// every initial-admin capability permanently seals the trail with no admin access possible.
    /// Requires the `RevokeCapabilities` permission. Emits a `CapabilityRevoked` event on
    /// success.
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
    /// Only prunes denylist entries whose stored `validUntil` is non-zero and strictly less than
    /// the current clock time. Entries with `validUntil == 0` (revocations without a known
    /// expiry) remain on the denylist indefinitely. Does not revoke additional capabilities and
    /// does not destroy any objects. Requires the `RevokeCapabilities` permission. Emits a
    /// `RevokedCapabilitiesCleanedUp` event on success.
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
    /// Returns the writable client for role mutations.
    ///
    /// Throws when this wrapper was created from `AuditTrailClientReadOnly`.
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
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Builds a role-creation transaction.
    ///
    /// Creates this role with `permissions` and the optional `roleTags` allowlist. Each tag
    /// referenced by `roleTags` must already exist in the trail-owned tag registry; the on-chain
    /// call aborts otherwise and bumps that tag's usage counter on success. Requires the
    /// `AddRoles` permission. Emits a `RoleCreated` event on success.
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
    /// The resulting capability always targets this trail and grants exactly this role. The
    /// `issuedTo`, `validFromMs`, and `validUntilMs` options on `WasmCapabilityIssueOptions` only
    /// configure restrictions on the issued object; enforcement happens on-chain when the
    /// capability is later presented for authorization. The capability is transferred to
    /// `issuedTo` if set, otherwise to the caller. Requires the `AddCapabilities` permission.
    /// Emits a `CapabilityIssued` event on success.
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
    /// Replaces both the role's permission set and its `roleTags` allowlist. Any newly supplied
    /// tag must already exist in the trail's record-tag registry; tag usage counters are adjusted
    /// to reflect the difference between the old and the new role-tag sets. Updating the
    /// initial-admin role with permissions that do not include every permission configured in
    /// the trail's role- and capability-admin permission sets aborts on-chain. Requires the
    /// `UpdateRoles` permission. Emits a `RoleUpdated` event on success.
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
    /// Decrements the usage count of every tag the role's `roleTags` referenced. The reserved
    /// initial-admin role cannot be deleted. Requires the `DeleteRoles` permission. Emits a
    /// `RoleDeleted` event on success.
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
