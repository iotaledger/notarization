// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Role-based access control capabilities for audit trails
module audit_trail::capability;

/// Capability granting role-based access to an audit trail
public struct Capability has key, store {
    id: UID,
}

/// Create a setup capability for trail initialization
public fun new_setup_cap(ctx: &mut TxContext): Capability {
    Capability {
        id: object::new(ctx),
    }
}

/// Create a new capability with a specific role
public fun new_capability(ctx: &mut TxContext): Capability {
    Capability {
        id: object::new(ctx),
    }
}

/// Get the capability's ID
public fun cap_id(cap: &Capability): ID {
    object::uid_to_inner(&cap.id)
}

/// Destroy a capability
public fun destroy_capability(cap: Capability) {
    let Capability { id } = cap;
    object::delete(id);
}
