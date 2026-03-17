// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use audit_trails::AuditTrailClient;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result};
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::{into_transaction_builder, parse_wasm_object_id};
use product_common::bindings::WasmObjectID;
use wasm_bindgen::prelude::*;

use crate::trail::{
    WasmCreateRole, WasmDeleteRole, WasmDestroyCapability, WasmDestroyInitialAdminCapability, WasmIssueCapability,
    WasmRevokeCapability, WasmRevokeInitialAdminCapability, WasmUpdateRole,
};
use crate::types::{WasmCapabilityIssueOptions, WasmPermissionSet};

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
    #[wasm_bindgen(js_name = forRole)]
    pub fn for_role(&self, name: String) -> WasmRoleHandle {
        WasmRoleHandle {
            full: self.full.clone(),
            trail_id: self.trail_id,
            name,
        }
    }

    #[wasm_bindgen(js_name = revokeCapability, unchecked_return_type = "TransactionBuilder<RevokeCapability>")]
    pub fn revoke_capability(&self, capability_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
        let capability_id = parse_wasm_object_id(&capability_id)?;
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .revoke_capability(capability_id)
            .into_inner();
        Ok(into_transaction_builder(WasmRevokeCapability(tx)))
    }

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

    #[wasm_bindgen(js_name = revokeInitialAdminCapability, unchecked_return_type = "TransactionBuilder<RevokeInitialAdminCapability>")]
    pub fn revoke_initial_admin_capability(&self, capability_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
        let capability_id = parse_wasm_object_id(&capability_id)?;
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .revoke_initial_admin_capability(capability_id)
            .into_inner();
        Ok(into_transaction_builder(WasmRevokeInitialAdminCapability(tx)))
    }
}

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
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<CreateRole>")]
    pub fn create(&self, permissions: WasmPermissionSet) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .for_role(self.name.clone())
            .create(permissions.into())
            .into_inner();
        Ok(into_transaction_builder(WasmCreateRole(tx)))
    }

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

    #[wasm_bindgen(js_name = updatePermissions, unchecked_return_type = "TransactionBuilder<UpdateRole>")]
    pub fn update_permissions(&self, permissions: WasmPermissionSet) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .access()
            .for_role(self.name.clone())
            .update_permissions(permissions.into())
            .into_inner();
        Ok(into_transaction_builder(WasmUpdateRole(tx)))
    }

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
