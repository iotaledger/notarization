// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Permission system for role-based access control
module audit_trails::permissions;

use iota::vec_set::{Self, VecSet};

public struct Permission has copy, drop, store {
    value: u8,
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

/// Permission to manage roles and permissions
public fun permission_admin(): Permission {
    Permission { value: 0 }
}

/// Permission to issue/revoke capabilities
public fun cap_admin(): Permission {
    Permission { value: 0 }
}

/// Permission to add records to the trail
public fun record_add(): Permission {
    Permission { value: 0 }
}

/// Permission to update trail metadata
public fun metadata_update(): Permission {
    Permission { value: 0 }
}

/// Permission to update locking configuration
public fun locking_update(): Permission {
    Permission { value: 0 }
}
