// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Role-based access control capabilities for audit trails
module audit_trail::capability;

use std::string::String;

// ===== Core Structures =====

/// Capability granting role-based access to an audit trail
public struct Capability has key, store {
    id: UID,
    trail_id: ID,
    role: String
}

/// Create a new capability with a specific role
public(package) fun new_capability(
    role: String,
    trail_id: ID,
    ctx: &mut TxContext,
): Capability { 
    Capability {        
        id: object::new(ctx),
        role,
        trail_id,
    }
}


// TODO: Is this needed? What is a setup capability?
//
// /// Create a setup capability for trail initialization
// public fun new_setup_cap(ctx: &mut TxContext): Capability {
//     Capability {
//         id: object::new(ctx),
//     }
// }



/// Get the capability's ID
public fun cap_id(cap: &Capability): ID {
    object::uid_to_inner(&cap.id)
}

/// Get the capability's role
public fun cap_role(cap: &Capability): &String {
    &cap.role
}

/// Get the capability's trail ID
public fun cap_trail_id(cap: &Capability): ID {
    cap.trail_id
}

/// Check if the capability has a specific role
public fun cap_has_role(cap: &Capability, role: &String): bool {
    &cap.role == role
}

/// Destroy a capability
public(package) fun cap_destroy(cap: Capability) {
    let Capability { id, role: _role, trail_id: _trail_id } = cap;
    object::delete(id);
}

// ===== public use statements =====

public use fun cap_id as Capability.id;
public use fun cap_role as Capability.role;
public use fun cap_trail_id as Capability.trail_id;
public use fun cap_has_role as Capability.has_role;
public use fun cap_destroy as Capability.destroy;