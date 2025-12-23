// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Permission system for role-based access control
module audit_trail::permission;

use iota::vec_set::{Self, VecSet};

/// Existing permissions for the Audit Trail object
public enum Permission has copy, drop, store {
    // --- Whole AUdit TRail related - Proposed role: `Admin` ---
    /// Destroy the whole Audit Trail object
    AuditTrailDelete,

    // --- Record Management - Proposed role: `RecordAdmin` ---
    /// Add records to the trail
    RecordAdd,
    /// Delete records from the trail
    RecordDelete,
    /// Correct existing records in the trail
    RecordCorrect, // TODO: Clarify if needed for MVP


    // --- Locking Config - Proposed role: `LockingAdmin` ---
    /// Edit the delete_lock configuration for records
    RecordDeleteLockConfig,
    /// Edit the delete_lock configuration for the whole Audit Trail
    TrailDeleteLockConfig,


    // --- Role Management - Proposed role: `RoleAdmin` ---
    /// Add new roles with associated permissions
    RolesAdd,
    /// Update permissions associated with existing roles
    RolesUpdate,
    /// Delete existing roles
    RolesDelete,
    
    // --- Capability Management - Proposed role: `CapAdmin` ---
    /// Issue new capabilities
    CapabilitiesAdd,
    /// Revoke existing capabilities
    CapabilitiesRevoke,

    // --- Meta Data related - Proposed role: `MetadataAdmin` ---
    /// Update the updatable metadata field
    MetadataUpdate,
    /// Delete the updatable metadata field
    MetadataDelete,
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

/// Create permissions typical used for the `Admin` rolepermissions
public fun admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(audit_trail_delete());
    perms.insert(capabilities_add());   
    perms.insert(capabilities_revoke());
    perms.insert(roles_add());
    perms.insert(roles_update());
    perms.insert(roles_delete());
    perms
}

/// Create permissions typical used for the `RecordAdmin` role
public fun record_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(record_add());
    perms.insert(record_delete());
    perms.insert(record_correct());
    perms
}

/// Create permissions typical used for the `LockingAdmin` role
public fun locking_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(record_delete_lock_config());
    perms.insert(trail_delete_lock_config());
    perms
}

/// Create permissions typical used for the `RoleAdmin` role
public fun role_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(roles_add());
    perms.insert(roles_update());
    perms.insert(roles_delete());
    perms
}

/// Create permissions typical used for the `CapAdmin` role
public fun cap_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(capabilities_add());
    perms.insert(capabilities_revoke());
    perms
}

/// Create permissions typical used for the `MetadataAdmin` role
public fun metadata_admin_permissions(): VecSet<Permission> {
    let mut perms = vec_set::empty();
    perms.insert(meta_data_update());
    perms.insert(meta_data_delete());
    perms
}

// --------------------------- Constructor functions for all Permission variants ---------------------------

/// Returns a permission allowing to destroy the whole Audit Trail object
public fun audit_trail_delete(): Permission {
    Permission::AuditTrailDelete
}

/// Returns a permission allowing to add records to the trail
public fun record_add(): Permission {
    Permission::RecordAdd
}

/// Returns a permission allowing to delete records from the trail
public fun record_delete(): Permission {
    Permission::RecordDelete
}

/// Returns a permission allowing to correct existing records in the trail
public fun record_correct(): Permission {
    Permission::RecordCorrect
}

/// Returns a permission allowing to edit the delete_lock configuration for records
public fun record_delete_lock_config(): Permission {
    Permission::RecordDeleteLockConfig
}

/// Returns a permission allowing to edit the delete_lock configuration for the whole Audit Trail
public fun trail_delete_lock_config(): Permission {
    Permission::TrailDeleteLockConfig
}

/// Returns a permission allowing to add new roles with associated permissions
public fun roles_add(): Permission {
    Permission::RolesAdd
}

/// Returns a permission allowing to update permissions associated with existing roles
public fun roles_update(): Permission {
    Permission::RolesUpdate
}

/// Returns a permission allowing to delete existing roles
public fun roles_delete(): Permission {
    Permission::RolesDelete
}

/// Returns a permission allowing to issue new capabilities
public fun capabilities_add(): Permission {
    Permission::CapabilitiesAdd
}

/// Returns a permission allowing to revoke existing capabilities
public fun capabilities_revoke(): Permission {
    Permission::CapabilitiesRevoke
}

/// Returns a permission allowing to update the updatable_metadata field
public fun meta_data_update(): Permission {
    Permission::MetadataUpdate
}

/// Returns a permission allowing to delete the updatable_metadata field
public fun meta_data_delete(): Permission {
    Permission::MetadataDelete
}
