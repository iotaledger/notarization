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

/// Convenience wrapper for permission sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionSet {
    pub permissions: Vec<Permission>,
}

impl PermissionSet {
    pub fn empty() -> Self {
        Self { permissions: vec![] }
    }

    pub fn from_vec(permissions: Vec<Permission>) -> Self {
        Self { permissions }
    }

    pub fn admin_permissions() -> Self {
        Self::from_vec(vec![
            Permission::DeleteAuditTrail,
            Permission::AddCapabilities,
            Permission::RevokeCapabilities,
            Permission::AddRoles,
            Permission::UpdateRoles,
            Permission::DeleteRoles,
        ])
    }

    pub fn record_admin_permissions() -> Self {
        Self::from_vec(vec![
            Permission::AddRecord,
            Permission::DeleteRecord,
            Permission::CorrectRecord,
        ])
    }

    pub fn locking_admin_permissions() -> Self {
        Self::from_vec(vec![
            Permission::UpdateLockingConfig,
            Permission::UpdateLockingConfigForDeleteTrail,
            Permission::UpdateLockingConfigForDeleteRecord,
        ])
    }

    pub fn role_admin_permissions() -> Self {
        Self::from_vec(vec![
            Permission::AddRoles,
            Permission::UpdateRoles,
            Permission::DeleteRoles,
        ])
    }

    pub fn cap_admin_permissions() -> Self {
        Self::from_vec(vec![Permission::AddCapabilities, Permission::RevokeCapabilities])
    }

    pub fn metadata_admin_permissions() -> Self {
        Self::from_vec(vec![Permission::UpdateMetadata, Permission::DeleteMetadata])
    }
}
