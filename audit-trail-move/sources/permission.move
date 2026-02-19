// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Permission system for role-based access control
module audit_trail::permission;

use iota::vec_set::{Self, VecSet};

/// Existing permissions for the Audit Trail object
public enum Permission has copy, drop, store {
    // --- Whole Audit Trail related - Proposed role: `Admin` ---
    /// Destroy the whole Audit Trail object
    DeleteAuditTrail,
    // --- Record Management - Proposed role: `RecordAdmin` ---
    /// Add records to the trail
    AddRecord,
    /// Delete records from the trail
    DeleteRecord,
    /// Correct existing records in the trail
    CorrectRecord,
    // --- Locking Config - Proposed role: `LockingAdmin` ---
    /// Update the whole locking configuration
    UpdateLockingConfig,
    /// Update the delete_record_lock configuration which is part of the locking configuration
    UpdateLockingConfigForDeleteRecord,
    /// Update the delete_lock configuration for the whole Audit Trail
    UpdateLockingConfigForDeleteTrail,
    // --- Role Management - Proposed role: `RoleAdmin` ---
    /// Add new roles with associated permissions
    AddRoles,
    /// Update permissions associated with existing roles
    UpdateRoles,
    /// Delete existing roles
    DeleteRoles,
    // --- Capability Management - Proposed role: `CapAdmin` ---
    /// Issue new capabilities
    AddCapabilities,
    /// Revoke existing capabilities
    RevokeCapabilities,
    // --- Meta Data related - Proposed role: `MetadataAdmin` ---
    /// Update the updatable metadata field
    UpdateMetadata,
    /// Delete the updatable metadata field
    DeleteMetadata,
    /// Migrate the audit trail to a new version of the contract
    Migrate,
}

/// Create an empty permission set
public fun empty(): VecSet<Permission> {
    vec_set::empty()
}

/// Add a permission to a set
public fun add(set: &mut VecSet<Permission>, perm: Permission) {
    vec_set::insert(set, perm);
}

/// Create a permission set from a vector
public fun from_vec(perms: vector<Permission>): VecSet<Permission> {
    let mut set = vec_set::empty();
    let mut i = 0;
    let len = perms.length();
    while (i < len) {
        vec_set::insert(&mut set, perms[i]);
        i = i + 1;
    };
    set
}

/// Check if a set contains a specific permission
public fun has_permission(set: &VecSet<Permission>, perm: &Permission): bool {
    vec_set::contains(set, perm)
}

// --------------------------- Functions creating permission sets for often used roles ---------------------------

/// Create permissions typically used for the `Admin` role
public fun admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(delete_audit_trail());
    perms.insert(add_capabilities());
    perms.insert(revoke_capabilities());
    perms.insert(add_roles());
    perms.insert(update_roles());
    perms.insert(delete_roles());
    perms
}

/// Create permissions typical used for the `RecordAdmin` role
public fun record_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(add_record());
    perms.insert(delete_record());
    perms.insert(correct_record());
    perms
}

/// Create permissions typical used for the `LockingAdmin` role
public fun locking_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(update_locking_config());
    perms.insert(update_locking_config_for_delete_trail());
    perms.insert(update_locking_config_for_delete_record());
    perms
}

/// Create permissions typical used for the `RoleAdmin` role
public fun role_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(add_roles());
    perms.insert(update_roles());
    perms.insert(delete_roles());
    perms
}

/// Create permissions typical used for the `CapAdmin` role
public fun cap_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(add_capabilities());
    perms.insert(revoke_capabilities());
    perms
}

/// Create permissions typical used for the `MetadataAdmin` role
public fun metadata_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(update_metadata());
    perms.insert(delete_metadata());
    perms
}

// ------- Constructor functions for all Permission variants -------------

/// Returns a permission allowing to destroy the whole Audit Trail object
public fun delete_audit_trail(): Permission {
    Permission::DeleteAuditTrail
}

/// Returns a permission allowing to add records to the trail
public fun add_record(): Permission {
    Permission::AddRecord
}

/// Returns a permission allowing to delete records from the trail
public fun delete_record(): Permission {
    Permission::DeleteRecord
}

/// Returns a permission allowing to correct existing records in the trail
public fun correct_record(): Permission {
    Permission::CorrectRecord
}

/// Returns a permission allowing to update the whole locking configuration
public fun update_locking_config(): Permission {
    Permission::UpdateLockingConfig
}

/// Returns a permission allowing to update the delete_lock configuration for records
public fun update_locking_config_for_delete_record(): Permission {
    Permission::UpdateLockingConfigForDeleteRecord
}

/// Returns a permission allowing to update the delete_lock configuration for the whole Audit Trail
public fun update_locking_config_for_delete_trail(): Permission {
    Permission::UpdateLockingConfigForDeleteTrail
}

/// Returns a permission allowing to add new roles with associated permissions
public fun add_roles(): Permission {
    Permission::AddRoles
}

/// Returns a permission allowing to update permissions associated with existing roles
public fun update_roles(): Permission {
    Permission::UpdateRoles
}

/// Returns a permission allowing to delete existing roles
public fun delete_roles(): Permission {
    Permission::DeleteRoles
}

/// Returns a permission allowing to issue new capabilities
public fun add_capabilities(): Permission {
    Permission::AddCapabilities
}

/// Returns a permission allowing to revoke existing capabilities
public fun revoke_capabilities(): Permission {
    Permission::RevokeCapabilities
}

/// Returns a permission allowing to update the updatable_metadata field
public fun update_metadata(): Permission {
    Permission::UpdateMetadata
}

/// Returns a permission allowing to delete the updatable_metadata field
public fun delete_metadata(): Permission {
    Permission::DeleteMetadata
}

/// Returns a permission allowing to migrate the audit trail to a new version of the contract
public fun migrate_audit_trail(): Permission {
    Permission::Migrate
}