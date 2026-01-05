// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Role-based access control capabilities for audit trails
module audit_trail::capability;

use std::string::String;
use iota::clock::{Self, Clock};

// ===== Errors =====

#[error]
const EValidityPeriodInconsistent: vector<u8> = b"Validity period is inconsistent: valid_from must be before valid_until";

// ===== Core Structures =====

/// Capability granting role-based access to a managed onchain object (i.e. an audit trail)
public struct Capability has key, store {
    id: UID,
    /// The ID of the onchain object this capability applies to
    security_vault_id: ID,
    /// The role granted by this capability
    /// Arbitrary string specifying a role contained in the `role_map::RoleMap` mapping
    role: String,
    /// For whom has this capability been issued
    /// * If Some(address), the capability is bound to that specific address
    /// * If None, the capability is not bound to a specific address
    issued_to: Option<address>,
    /// Optional validity period start timestamp (in seconds since Unix epoch)
    /// * The specified timestamp is included in the validity period
    /// * If None, the capability is valid from creation time
    valid_from: Option<u64>,
    /// Optional validity period end timestamp (in seconds since Unix epoch)
    /// * The specified timestamp is excluded in the validity period
    /// * If None, the capability does not expire
    valid_until: Option<u64>,
}

/// Create a new capability with a specific role and all available optional restrictions
///
/// Parameters:
/// * role: The role granted by this capability
/// * security_vault_id: The ID of onchain object (i.e. an audit trail) this capability applies to
/// * issued_to: Optional address restriction; if Some(address), the capability is bound to that specific address
/// * valid_from: Optional validity period start timestamp (in seconds since Unix epoch); if Some(ts), the capability is valid from that timestamp onwards
/// * valid_until: Optional validity period end timestamp (in seconds since Unix epoch); if Some(ts), the capability is valid until that timestamp
/// * ctx: The transaction context
///
/// Returns: The newly created Capability
///
/// Errors:
/// * EValidityPeriodInconsistent: If both valid_from and valid_until are provided and valid_from >= valid_until
public(package) fun new_capability(
    role: String,
    security_vault_id: ID,
    issued_to: Option<address>,
    valid_from: Option<u64>,
    valid_until: Option<u64>,
    ctx: &mut TxContext,
): Capability {
    if (valid_from.is_some() && valid_until.is_some()) {
        let from = valid_from.borrow();
        let until = valid_until.borrow();
        assert!(*from < *until, EValidityPeriodInconsistent);
    };
    Capability {
        id: object::new(ctx),
        role,
        security_vault_id,
        issued_to,
        valid_from,
        valid_until,
    }
}

/// Create a new unrestricted capability with a specific role
public(package) fun new_capability_without_restrictions(
    role: String,
    security_vault_id: ID,
    ctx: &mut TxContext,
): Capability {
    Capability {
        id: object::new(ctx),
        role,
        security_vault_id,
        issued_to: std::option::none(),
        valid_from: std::option::none(),
        valid_until: std::option::none(),
    }
}

/// Create a new capability with a specific role and validity period, valid until the given timestamp
public(package) fun new_capability_valid_until(
    role: String,
    security_vault_id: ID,
    valid_until: u64,
    ctx: &mut TxContext,
): Capability {
    Capability {
        id: object::new(ctx),
        role,
        security_vault_id,
        issued_to: std::option::none(),
        valid_from: std::option::none(),
        valid_until: std::option::some(valid_until),
    }
}

/// Create a new capability with a specific role, exclusively usable by a specific address and an optional
/// validity period, valid until the given timestamp
public(package) fun new_capability_for_address(
    role: String,
    security_vault_id: ID,
    issued_to: address,
    valid_until: Option<u64>,
    ctx: &mut TxContext,
): Capability {
    Capability {
        id: object::new(ctx),
        role,
        security_vault_id,
        issued_to: std::option::some(issued_to),
        valid_from: std::option::none(),
        valid_until,
    }
}

/// Get the capability's ID
public fun cap_id(cap: &Capability): ID {
    object::uid_to_inner(&cap.id)
}

/// Get the capability's role
public fun cap_role(cap: &Capability): &String {
    &cap.role
}

/// Get the capability's security_vault_id
public fun cap_security_vault_id(cap: &Capability): ID {
    cap.security_vault_id
}

/// Check if the capability has a specific role
public fun cap_has_role(cap: &Capability, role: &String): bool {
    &cap.role == role
}

// Get the capability's issued_to address
public fun cap_issued_to(cap: &Capability): &Option<address> {
    &cap.issued_to
}

// Get the capability's valid_from timestamp
public fun cap_valid_from(cap: &Capability): &Option<u64> {
    &cap.valid_from
}

// Get the capability's valid_until timestamp
public fun cap_valid_until(cap: &Capability): &Option<u64> {
    &cap.valid_until
}

// Check if the capability is currently valid for `clock::timestamp_ms(clock)`
public fun cap_is_currently_valid(cap: &Capability, clock: &Clock,): bool {
    let current_ts = clock::timestamp_ms(clock) / 1000; // convert to seconds
    cap.is_valid_for_timestamp(current_ts)
}

// Check if the capability is valid for a specific timestamp (in seconds since Unix epoch)
public fun cap_is_valid_for_timestamp(cap: &Capability, timestamp_secs: u64,): bool {
    let valid_from_ok = if (cap.valid_from.is_some()) {
        let from = cap.valid_from.borrow();
        timestamp_secs >= *from
    } else  {
        true
    };
    let valid_until_ok = if (cap.valid_until.is_some()) {
        let until = cap.valid_until.borrow();
        timestamp_secs < *until
    } else  {
        true
    };
    valid_from_ok && valid_until_ok
}

/// Destroy a capability
public(package) fun cap_destroy(cap: Capability) {
    let Capability {
        id,
        role: _role,
        security_vault_id: _trail_id,
        issued_to: _issued_to,
        valid_from: _valid_from,
        valid_until: _valid_until,
    } = cap;
    object::delete(id);
}

#[test_only]
public fun cap_destroy_for_testing(cap: Capability) {
    cap_destroy(cap);
}

// ===== public use statements =====

public use fun cap_id as Capability.id;
public use fun cap_role as Capability.role;
public use fun cap_security_vault_id as Capability.security_vault_id;
public use fun cap_has_role as Capability.has_role;
public use fun cap_destroy as Capability.destroy;
public use fun cap_issued_to as Capability.issued_to;
public use fun cap_valid_from as Capability.valid_from;
public use fun cap_valid_until as Capability.valid_until;
public use fun cap_is_currently_valid as Capability.is_currently_valid;
public use fun cap_is_valid_for_timestamp as Capability.is_valid_for_timestamp;
#[test_only]
public use fun cap_destroy_for_testing as Capability.destroy_for_testing;
