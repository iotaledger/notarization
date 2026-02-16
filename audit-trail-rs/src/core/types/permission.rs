// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// Permission enum matching the Move permission module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Permission {
    DeleteAuditTrail,
    AddRecord,
    DeleteRecord,
    CorrectRecord,
    UpdateLockingConfig,
    UpdateLockingConfigForDeleteRecord,
    UpdateLockingConfigForDeleteTrail,
    AddRoles,
    UpdateRoles,
    DeleteRoles,
    AddCapabilities,
    RevokeCapabilities,
    UpdateMetadata,
    DeleteMetadata,
}

impl Permission {
    /// Returns the Move constructor function name for this permission variant.
    pub(crate) fn move_function_name(&self) -> &'static str {
        match self {
            Self::DeleteAuditTrail => "delete_audit_trail",
            Self::AddRecord => "add_record",
            Self::DeleteRecord => "delete_record",
            Self::CorrectRecord => "correct_record",
            Self::UpdateLockingConfig => "update_locking_config",
            Self::UpdateLockingConfigForDeleteRecord => "update_locking_config_for_delete_record",
            Self::UpdateLockingConfigForDeleteTrail => "update_locking_config_for_delete_trail",
            Self::AddRoles => "add_roles",
            Self::UpdateRoles => "update_roles",
            Self::DeleteRoles => "delete_roles",
            Self::AddCapabilities => "add_capabilities",
            Self::RevokeCapabilities => "revoke_capabilities",
            Self::UpdateMetadata => "update_metadata",
            Self::DeleteMetadata => "delete_metadata",
        }
    }
}

/// Convenience wrapper for permission sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionSet {
    pub permissions: Vec<Permission>,
}

impl PermissionSet {
    pub fn empty() -> Self {
        Self { permissions: vec![] }
    }

    pub fn admin_permissions() -> Self {
        Self {
            permissions: vec![
                Permission::DeleteAuditTrail,
                Permission::AddCapabilities,
                Permission::RevokeCapabilities,
                Permission::AddRoles,
                Permission::UpdateRoles,
                Permission::DeleteRoles,
            ],
        }
    }

    pub fn record_admin_permissions() -> Self {
        Self {
            permissions: vec![
                Permission::AddRecord,
                Permission::DeleteRecord,
                Permission::CorrectRecord,
            ],
        }
    }

    pub fn locking_admin_permissions() -> Self {
        Self {
            permissions: vec![
                Permission::UpdateLockingConfig,
                Permission::UpdateLockingConfigForDeleteTrail,
                Permission::UpdateLockingConfigForDeleteRecord,
            ],
        }
    }

    pub fn role_admin_permissions() -> Self {
        Self {
            permissions: vec![Permission::AddRoles, Permission::UpdateRoles, Permission::DeleteRoles],
        }
    }

    pub fn cap_admin_permissions() -> Self {
        Self {
            permissions: vec![Permission::AddCapabilities, Permission::RevokeCapabilities],
        }
    }

    pub fn metadata_admin_permissions() -> Self {
        Self {
            permissions: vec![Permission::UpdateMetadata, Permission::DeleteMetadata],
        }
    }
}
