// Copyright (c) 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// A role-based access control helper mapping unique role identifiers to their associated permissions.
///
/// Provides the following functionalities:
/// - Define an initial role with a custom set of permissions (i.e. an Admin role).
/// - Use custom permission types defined by the integrating module using the generic parameter `P`.
/// - Create, delete, and update roles and their permissions 
/// - Issue, revoke, and destroy `audit_trail::capability`s associated with a specific role.
/// - Validate `audit_trail::capability`s against the defined roles to facilitate proper access control by other modules
///   (function `RoleMap.is_capability_valid()`)
/// - All functions are access restricted by custom permissions defined during `RoleMap` instantiation.
///
/// Examples:
/// - audit_trail::main module uses `RoleMap` to manage access to the audit trail records and their operations.
 
module audit_trail::role_map;

use std::string::String;
use iota::vec_map::{Self, VecMap};
use iota::vec_set::{Self, VecSet};
use audit_trail::capability::{Self, Capability};
use iota::clock::Clock;
use iota::event;

// =============== Errors ==========================================================

#[error]
const EPermissionDenied: vector<u8> = b"The role associated with the provided capability does not have the required permission";
#[error]
const ERoleDoesNotExist: vector<u8> = b"The specified role, directly specified or specified by a capability, does not exist in the `RoleMap` mapping";
#[error]
const ECapabilityHasBeenRevoked: vector<u8> = b"The provided capability has been revoked and is no longer valid";
#[error]
const ECapabilitySecurityVaultIdMismatch: vector<u8> = b"The security_vault_id associated with the provided capability does not match the security_vault_id of the `RoleMap`";
#[error]
const ECapabilityTimeConstraintsNotMet: vector<u8> = b"The capability's time constraints are not currently met either due to `valid_from` or `valid_until` restrictions";
#[error]
const ECapabilityIssuedToMismatch: vector<u8> = b"The capability is restricted to a specific address which does not match the caller's address";
#[error]
const ECapabilityPermissionDenied: vector<u8> = b"The role associated with provided capability does not have the required permission";


// =============== Events ==========================================================

/// Emitted when a capability is issued
public struct CapabilityIssued has copy, drop {
    security_vault_id: ID,
    capability_id: ID,
    role: String,
    issued_to: Option<address>,
    valid_from: Option<u64>,
    valid_until: Option<u64>,
}

/// Emitted when a capability is destroyed
public struct CapabilityDestroyed has copy, drop {
    security_vault_id: ID,
    capability_id: ID,
    role: String,
    issued_to: Option<address>,
    valid_from: Option<u64>,
    valid_until: Option<u64>,
}

/// Emitted when a capability is revoked or destroyed
public struct CapabilityRevoked has copy, drop {
    security_vault_id: ID,
    capability_id: ID,
} 

// TODO: Add event for Role creation, removing, updating, etc.


// =============== Core Types ======================================================

/// Defines the permissions required to administer roles in this RoleMap
public struct RoleAdminPermissions<P: copy + drop> has copy, drop, store {
    /// Permission required to add a new role
    add: P,
    /// Permission required to delete an existing role
    delete: P,
    /// Permission required to update permissions associated with an existing role
    update: P,
}

/// Defines the permissions required to administer capabilities in this RoleMap
public struct CapabilityAdminPermissions<P: copy + drop> has copy, drop, store {
    /// Permission required to add (issue) a new capability
    add: P,
    /// Permission required to revoke an existing capability
    revoke: P,
}

/// The RoleMap structure mapping role names to their associated permissions
/// Generic parameter P defines the permission type used by the integrating module
/// (i.e. audit_trail::Permission)
public struct RoleMap<P: copy + drop> has copy, drop, store {
    /// The ObjectID of the onchain object integrating this RoleMap
    security_vault_id: ID,
    /// Mapping of role names to their associated permissions
    roles: VecMap<std::string::String, VecSet<P>>,
    /// Whitelist of all issued capability IDs
    issued_capabilities: VecSet<ID>,
    /// Permissions required to administer roles in this RoleMap
    role_admin_permissions: RoleAdminPermissions<P>,
    /// Permissions required to administer capabilities in this RoleMap
    capability_admin_permissions: CapabilityAdminPermissions<P>,
}

// =============== Role & Capability AdminPermissions Functions ====================

public fun new_role_admin_permissions<P: copy + drop>(
    add: P,
    delete: P,
    update: P,
): RoleAdminPermissions<P> {
    RoleAdminPermissions {
        add,
        delete,
        update,
    }
}

public fun new_capability_admin_permissions<P: copy + drop>(
    add: P,
    revoke: P,
): CapabilityAdminPermissions<P> {
    CapabilityAdminPermissions {
        add,
        revoke,
    }
}

// =============== RoleMap Functions ===============================================

/// Create a new RoleMap with an initial admin role
/// The initial admin role is created with the specified name and permissions
/// An initial admin capability is created and returned alongside the RoleMap
/// The initial admin capability has no restrictions (no address, valid_from, or valid_until)
/// The security_vault_id is associated with both the RoleMap and the initial admin capability
/// Returns the newly created RoleMap and the initial admin capability
/// 
/// Parameters
/// ----------
/// - security_vault_id:
///   The security_vault_id to associate this RoleMap with the initial admin capability
///   and all other created capabilities. Set this to the ID of the onchain object that integrates the RoleMap.
/// - initial_admin_role_name:
///   The name of the initial admin role
/// - initial_admin_role_permissions:
///   The permissions associated with the initial admin role
/// - role_admin_permissions:
///   The permissions required to administer roles in this RoleMap
/// - capability_admin_permissions:
///   The permissions required to administer capabilities in this RoleMap
/// - ctx:
///   The transaction context for capability creation
public fun new<P: copy + drop>(
    security_vault_id: ID,
    initial_admin_role_name: String,
    initial_admin_role_permissions: VecSet<P>,
    role_admin_permissions: RoleAdminPermissions<P>,
    capability_admin_permissions: CapabilityAdminPermissions<P>,
    ctx: &mut TxContext,
): (RoleMap<P>, Capability) {
    let mut roles = vec_map::empty<String, VecSet<P>>();
    roles.insert(initial_admin_role_name, initial_admin_role_permissions);
        
    let admin_cap = capability::new_capability_without_restrictions(
        initial_admin_role_name,
        security_vault_id,
        ctx,
    );
    let mut issued_capabilities = vec_set::empty<ID>();
    issued_capabilities.insert(admin_cap.id());
    let role_map = RoleMap {
        roles,
        role_admin_permissions,
        capability_admin_permissions,
        security_vault_id,
        issued_capabilities,
    };

    (role_map, admin_cap)
}

/// Get the permissions associated with a specific role.
/// Aborts with ERoleDoesNotExist if the role does not exist.
public fun rmap_get_role_permissions<P: copy + drop>(
    role_map: &RoleMap<P>,
    role: &String,
): &VecSet<P> {
    assert!(vec_map::contains(&role_map.roles, role), ERoleDoesNotExist);
    vec_map::get(&role_map.roles, role)
}

/// Create a new role consisting of a role name and associated permissions
public fun rmap_create_role<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: String,
    permissions: VecSet<P>,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(role_map.is_capability_valid(
            cap,
            &role_map.role_admin_permissions.add,
            clock,
            ctx
        ),
        EPermissionDenied
    );    

    vec_map::insert(&mut role_map.roles, role, permissions);
}

/// Delete an existing role
public fun rmap_delete_role<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: &String,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(role_map.is_capability_valid(
            cap,
            &role_map.role_admin_permissions.delete,
            clock,
            ctx
        ),
        EPermissionDenied
    );    

    vec_map::remove(&mut role_map.roles, role);
}

/// Update permissions associated with an existing role
public fun rmap_update_role_permissions<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: &String,
    new_permissions: VecSet<P>,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(role_map.is_capability_valid(
            cap,
            &role_map.role_admin_permissions.update,
            clock,
            ctx
        ),
        EPermissionDenied
    );    

    assert!(vec_map::contains(&role_map.roles, role), ERoleDoesNotExist);
    vec_map::insert(&mut role_map.roles, *role, new_permissions);
}

/// Indicates if the specified role exists in the role_map
public fun rmap_has_role<P: copy + drop>(
    role_map: &RoleMap<P>,
    role: &String,
): bool {
    vec_map::contains(&role_map.roles, role)
}

// =============== Capability related Functions ====================================

/// Indicates if a provided capability is valid.
/// 
/// A capability is considered valid if:
/// - The capability's security_vault_id matches the RoleMap's security_vault_id.
///   Aborts with ECapabilitySecurityVaultIdMismatch if not matching.
/// - The role value specified by the capability exists in the `RoleMap` mapping.
///   Aborts with ERoleDoesNotExist if the role does not exist.
/// - The role associated with the capability contains the permission specified by the `permission` argument.
///   Aborts with ECapabilityPermissionDenied if the permission is not granted by the role.
/// - The capability has not been revoked (is included in the `issued_capabilities` set).
///   Aborts with ECapabilityHasBeenRevoked if revoked.
/// - The capability is currently active, based on its time restrictions (if any).
///   Aborts with ECapabilityTimeConstraintsNotMet, if the current time is outside the valid_from and valid_until range.
/// - If the capability is restricted to a specific address, the caller's address matches the sender of the transaction.
///   Aborts with ECapabilityIssuedToMismatch if the addresses do not match.
/// 
/// Parameters
/// ----------
/// - role_map: Reference to the `RoleMap` mapping.
/// - cap: Reference to the capability to be validated.
/// - permission: The permission to check against the capability's role.
/// - clock: Reference to a Clock instance for time-based validation.
/// - ctx: Reference to the transaction context for accessing the caller's address.
/// 
/// Returns
/// -------
/// - bool: true if the capability is valid, otherwise aborts with the relevant error.
public fun rmap_is_capability_valid<P: copy + drop>(
    role_map: &RoleMap<P>,
    cap: &Capability,
    permission: &P,
    clock: &Clock,
    ctx: &TxContext,
): bool {
    assert!(role_map.security_vault_id == cap.security_vault_id(), ECapabilitySecurityVaultIdMismatch);

    let permissions = role_map.get_role_permissions(cap.role());
    assert!(vec_set::contains(permissions, permission), ECapabilityPermissionDenied);

    assert!(role_map.issued_capabilities.contains(&cap.id()), ECapabilityHasBeenRevoked);
    
    if (cap.valid_from().is_some() || cap.valid_until().is_some()) {
        assert!(cap.is_currently_valid(clock), ECapabilityTimeConstraintsNotMet);
    };

    if (cap.issued_to().is_some()) {
        let caller = ctx.sender();
        let issued_to_addr = cap.issued_to().borrow();
        assert!(*issued_to_addr == caller, ECapabilityIssuedToMismatch);
    };

    true
}

/// Create a new capability
///
/// Parameters
/// ----------
/// - role_map: Reference to the `RoleMap` mapping.
/// - cap: Reference to the capability used to authorize the creation of the new capability.
/// - role: The role to be assigned to the new capability.
/// - issued_to: Optional address restriction for the new capability.
/// - valid_from: Optional start time (in seconds since Unix epoch) for the new capability.
/// - valid_until: Optional end time (in seconds since Unix epoch) for the new capability.
/// - clock: Reference to a Clock instance for time-based validation.
/// - ctx: Reference to the transaction context.
/// 
/// Returns the newly created capability.
/// 
/// Sends a CapabilityIssued event upon successful creation.
/// 
/// Errors:
/// - Aborts with EPermissionDenied if the provided capability does not have the permission specified with `CapabilityAdminPermissions::add`.
/// - Aborts with ERoleDoesNotExist if the specified role does not exist in the role_map.
/// - Aborts with audit_trail::capability::EValidityPeriodInconsistent if the provided valid_from and valid_until are inconsistent.
public fun rmap_new_capability<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: &String,
    issued_to: Option<address>,
    valid_from: Option<u64>,
    valid_until: Option<u64>,
    clock: &Clock,
    ctx: &mut TxContext,
): Capability {
    assert!(role_map.is_capability_valid(
            cap,
            &role_map.capability_admin_permissions.add,
            clock,
            ctx
        ),
        EPermissionDenied
    );    

    assert!(role_map.roles.contains(role), ERoleDoesNotExist);
    let new_cap = capability::new_capability(
        *role,
        role_map.security_vault_id,
        issued_to,
        valid_from,
        valid_until,
        ctx,
    );
    register_new_capability(role_map, &new_cap);
    new_cap
}

/// Create a new unrestricted capability with a specific role without any
/// address, valid_from, or valid_until restrictions.
/// 
/// Returns the newly created capability.
/// 
/// Sends a CapabilityIssued event upon successful creation.
/// 
/// Errors:
/// - Aborts with EPermissionDenied if the provided capability does not have the permission specified with `CapabilityAdminPermissions::add`.
/// - Aborts with ERoleDoesNotExist if the specified role does not exist in the role_map.
public fun rmap_new_capability_without_restrictions<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: &String,
    clock: &Clock,
    ctx: &mut TxContext,
): Capability {
    assert!(role_map.is_capability_valid(
            cap,
            &role_map.capability_admin_permissions.add,
            clock,
            ctx
        ),
        EPermissionDenied
    );    

    assert!(role_map.roles.contains(role), ERoleDoesNotExist);
    let new_cap = capability::new_capability_without_restrictions(
        *role,
        role_map.security_vault_id,
        ctx,
    );

    register_new_capability(role_map, &new_cap);
    new_cap
}

/// Create a new capability with a specific role that expires at a given timestamp (seconds since Unix epoch).
/// 
/// Returns the newly created capability.
/// 
/// Sends a CapabilityIssued event upon successful creation.
/// 
/// Errors:
/// - Aborts with EPermissionDenied if the provided capability does not have the permission specified with `CapabilityAdminPermissions::add`.
/// - Aborts with ERoleDoesNotExist if the specified role does not exist in the role_map.
public fun rmap_new_capability_valid_until<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: &String,
    valid_until: u64,
    clock: &Clock,
    ctx: &mut TxContext,
): Capability {
        assert!(role_map.is_capability_valid(
            cap,
            &role_map.capability_admin_permissions.add,
            clock,
            ctx
        ),
        EPermissionDenied
    );    

    assert!(role_map.roles.contains(role), ERoleDoesNotExist);
    let new_cap = capability::new_capability_valid_until(
        *role,
        role_map.security_vault_id,
        valid_until,
        ctx,
    );

    register_new_capability(role_map, &new_cap);
    new_cap
}

/// Create a new capability with a specific role restricted to an address.
/// Optionally set an expiration time (seconds since Unix epoch).
/// 
/// Returns the newly created capability.
/// 
/// Sends a CapabilityIssued event upon successful creation.
/// 
/// Errors:
/// - Aborts with EPermissionDenied if the provided capability does not have the permission specified with `CapabilityAdminPermissions::add`.
/// - Aborts with ERoleDoesNotExist if the specified role does not exist in the role_map.
public fun rmap_new_capability_for_address<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: &String,
    issued_to: address,
    valid_until: Option<u64>,
    clock: &Clock,
    ctx: &mut TxContext,
): Capability {
        assert!(role_map.is_capability_valid(
            cap,
            &role_map.capability_admin_permissions.add,
            clock,
            ctx
        ),
        EPermissionDenied
    );    

    assert!(role_map.roles.contains(role), ERoleDoesNotExist);
    let new_cap = capability::new_capability_for_address(
        *role,
        role_map.security_vault_id,
        issued_to,
        valid_until,
        ctx,
    );

    register_new_capability(role_map, &new_cap);
    new_cap
}

/// Destroy an existing capability
/// Every owner of a capability is allowed to destroy it when no longer needed.
/// 
/// Sends a CapabilityDestroyed event upon successful destruction.
/// 
/// TODO: Clarify if we need to restrict access with the `CapabilitiesRevoke` permission here.
///       If yes, we also need a destroy function for Admin capabilities (without the need of another Admin capability).
///       Otherwise the last Admin capability holder will block the role_map forever by not being able to destroy it.
public fun rmap_destroy_capability<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap_to_destroy: Capability,
) {
    assert!(role_map.security_vault_id == cap_to_destroy.security_vault_id(), ECapabilitySecurityVaultIdMismatch);
    role_map.issued_capabilities.remove(&cap_to_destroy.id());

    event::emit(CapabilityDestroyed {
        security_vault_id: role_map.security_vault_id,
        capability_id: cap_to_destroy.id(),
        role: *cap_to_destroy.role(),
        issued_to: *cap_to_destroy.issued_to(),
        valid_from: *cap_to_destroy.valid_from(),
        valid_until: *cap_to_destroy.valid_until(),
    });

    cap_to_destroy.destroy();
}

/// Revoke an existing capability
/// 
/// Sends a CapabilityRevoked event upon successful revocation.
/// 
/// Errors:
/// - Aborts with EPermissionDenied if the provided capability does not have the permission specified with `CapabilityAdminPermissions::revoke`.
/// - Aborts with ERoleDoesNotExist if the specified role does not exist in the `RoleMap.issued_capabilities()` list.
public fun rmap_revoke_capability<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    cap_to_revoke: ID,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(role_map.is_capability_valid(
            cap,
            &role_map.capability_admin_permissions.revoke,
            clock,
            ctx
        ),
        EPermissionDenied
    );

    assert!(role_map.issued_capabilities.contains(&cap_to_revoke), ERoleDoesNotExist);
    role_map.issued_capabilities.remove(&cap_to_revoke);

    event::emit(CapabilityRevoked {
        security_vault_id: role_map.security_vault_id,
        capability_id: cap_to_revoke,
    });
}

fun register_new_capability<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    new_cap: &Capability,
) {
    role_map.issued_capabilities.insert(new_cap.id());

    event::emit(CapabilityIssued {
        security_vault_id: role_map.security_vault_id,
        capability_id: new_cap.id(),
        role: *new_cap.role(),
        issued_to: *new_cap.issued_to(),
        valid_from: *new_cap.valid_from(),
        valid_until: *new_cap.valid_until(),
    });
}

// =============== Getter Functions ================================================

/// Returns the size of the role_map, the number of managed roles
public fun rmap_size<P: copy + drop>(role_map: &RoleMap<P>): u64 {
    vec_map::size(&role_map.roles)
}

/// Returns the security_vault_id associated with the role_map
public fun rmap_security_vault_id<P: copy + drop>(role_map: &RoleMap<P>): ID {
    role_map.security_vault_id
}

//Returns the role admin permissions associated with the role_map
public fun rmap_role_admin_permissions<P: copy + drop>(role_map: &RoleMap<P>): &RoleAdminPermissions<P> {
    &role_map.role_admin_permissions
}

public fun rmap_issued_capabilities<P: copy + drop>(
    role_map: &RoleMap<P>,
): &VecSet<ID> {
    &role_map.issued_capabilities
}

// =============== public use statements ===========================================

public use fun rmap_get_role_permissions as RoleMap.get_role_permissions;
public use fun rmap_create_role as RoleMap.create_role;
public use fun rmap_delete_role as RoleMap.delete_role;
public use fun rmap_update_role_permissions as RoleMap.update_role_permissions;
public use fun rmap_has_role as RoleMap.has_role;
public use fun rmap_size as RoleMap.size;
public use fun rmap_security_vault_id as RoleMap.security_vault_id;
public use fun rmap_role_admin_permissions as RoleMap.role_admin_permissions;
public use fun rmap_is_capability_valid as RoleMap.is_capability_valid;
public use fun rmap_new_capability as RoleMap.new_capability;
public use fun rmap_new_capability_without_restrictions as RoleMap.new_capability_without_restrictions;
public use fun rmap_new_capability_valid_until as RoleMap.new_capability_valid_until;
public use fun rmap_new_capability_for_address as RoleMap.new_capability_for_address;
public use fun rmap_destroy_capability as RoleMap.destroy_capability;
public use fun rmap_revoke_capability as RoleMap.revoke_capability;
public use fun rmap_issued_capabilities as RoleMap.issued_capabilities;
