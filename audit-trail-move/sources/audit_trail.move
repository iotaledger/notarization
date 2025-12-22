// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Audit Trails with role-based access control and timelock
///
/// An audit trail is a tamper-proof, sequential chain of notarized records where each entry
/// references its predecessor, ensuring verifiable continuity and integrity.
///
/// Records are addressed by trail_id + sequence_number
module audit_trail::main;

use audit_trail::capability::{Self, Capability};
use audit_trail::locking::{Self, LockingConfig};
use audit_trail::permission::{Self, Permission};
use audit_trail::record::{Self, Record};
use iota::clock::{Self, Clock};
use iota::event;
use iota::linked_table::{Self, LinkedTable};
use iota::vec_map::{Self, VecMap};
use iota::vec_set::{Self, VecSet};
use std::string::String;

// ===== Errors =====
#[error]
const ERecordNotFound: vector<u8> = b"Record not found at the given sequence number";
#[error]
const ERoleDoesNotExist: vector<u8> = b"The specified role does not exist in the roles map";
#[error]
const EPermissionDenied: vector<u8> = b"The role associated with the provided capability does not have the required permission";
#[error]
const ETrailIdNotCorrect: vector<u8> = b"The trail ID associated with the provided capability does not match the audit trail";

// ===== Constants =====
const INITIAL_ADMIN_ROLE_NAME: vector<u8> = b"Admin";

// ===== Core Structures =====

/// Metadata set at trail creation (immutable)
public struct TrailImmutableMetadata has copy, drop, store {
    name: Option<String>,
    description: Option<String>,
}

/// Shared audit trail object with role-based access control
/// Records are stored in a LinkedTable and addressed by sequence number
public struct AuditTrail<D: store + copy> has key, store {
    id: UID,
    /// Address that created this trail
    creator: address,
    /// Creation timestamp (milliseconds)
    created_at: u64,
    /// Total records ever added (also serves as next sequence number)
    record_count: u64,
    /// Records stored by sequence number (0-indexed)
    records: LinkedTable<u64, Record<D>>,
    /// Deletion locking rules
    locking_config: LockingConfig,
    /// A list of role definitions consisting of a unique role specifier and a list of associated permissions
    roles: VecMap<String, VecSet<Permission>>,
    /// Set at creation, cannot be changed
    immutable_metadata: TrailImmutableMetadata,
    /// Can be updated by holders of MetadataUpdate permission
    updatable_metadata: Option<String>,
    /// Whitelist of all issued capability IDs (TODO: implement)
    issued_capability: VecSet<ID>,
}

// ===== Events =====

/// Emitted when a new trail is created
public struct AuditTrailCreated has copy, drop {
    trail_id: ID,
    creator: address,
    timestamp: u64,
    has_initial_record: bool,
}

// TODO: Add event for trail deletion

/// Emitted when a record is added to the trail
/// Records are identified by trail_id + sequence_number
public struct RecordAdded has copy, drop {
    trail_id: ID,
    sequence_number: u64,
    added_by: address,
    timestamp: u64,
}

// TODO: Add event for Record deletion and (if part of MVP) correction

/// Emitted when a capability is issued
public struct CapabilityIssued has copy, drop {
    trail_id: ID,
    capability_id: ID,
    role: String,
    issued_to: address,
    issued_by: address,
    timestamp: u64,
}   


// ===== Constructors =====

/// Create immutable trail metadata
public fun new_trail_metadata(
    name: Option<String>,
    description: Option<String>,
): TrailImmutableMetadata {
    TrailImmutableMetadata { name, description }
}

// ===== Trail Creation =====

/// Create a new audit trail with optional initial record
/// 
/// Initial roles config
/// --------------------
/// Initializes the `roles` map with only one role, called "Admin" which is associated with the permissions
/// * TrailDelete
/// * CapabilitiesAdd
/// * CapabilitiesRevoke
/// * RolesAdd
/// * RolesUpdate
/// * RolesDelete
///
/// Returns
/// -------
/// * Capability with "Admin" role, allowing the creator to define custom
///   roles and issue capabilities to other users.
/// * Trail ID
public fun create<D: store + copy>(
    initial_data: Option<D>,
    initial_record_metadata: Option<String>,
    locking_config: LockingConfig,
    trail_metadata: TrailImmutableMetadata,
    updatable_metadata: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
): (Capability, ID) {
    let creator = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);

    let trail_uid = object::new(ctx);
    let trail_id = object::uid_to_inner(&trail_uid);

    let mut records = linked_table::new<u64, Record<D>>(ctx);
    let mut record_count = 0;
    let has_initial_record = initial_data.is_some();

    if (initial_data.is_some()) {
        let record = record::new(
            initial_data.destroy_some(),
            initial_record_metadata,
            0, // sequence_number
            creator,
            timestamp,
        );

        linked_table::push_back(&mut records, 0, record);
        record_count = 1;

        event::emit(RecordAdded {
            trail_id,
            sequence_number: 0,
            added_by: creator,
            timestamp,
        });
    } else {
        initial_data.destroy_none();
    };

    let mut roles = vec_map::empty<String, VecSet<Permission>>();
    roles.insert(initial_admin_role_name(), permission::admin_permissions());

    let trail = AuditTrail {
        id: trail_uid,
        creator,
        created_at: timestamp,
        record_count,
        records,
        locking_config,
        roles,
        immutable_metadata: trail_metadata,
        updatable_metadata,
        issued_capability: iota::vec_set::empty(),
    };

    transfer::share_object(trail);

    let admin_cap = capability::new_capability(
        initial_admin_role_name(),
        trail_id,
        ctx,
    );
    
    event::emit(AuditTrailCreated {
        trail_id,
        creator,
        timestamp,
        has_initial_record,
    });

    (admin_cap, trail_id)
}

public fun initial_admin_role_name(): String {
    INITIAL_ADMIN_ROLE_NAME.to_string()
}

// ===== Record Operations =====

/// Add a record to the trail
///
/// Records are added sequentially with auto-assigned sequence numbers.
///
/// TODO: Add capability parameter and permission check once implemented
public fun trail_add_record<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    stored_data: D,
    record_metadata: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(trail.has_capability_permission(cap, &permission::record_add()), EPermissionDenied);

    let caller = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = object::uid_to_inner(&trail.id);
    let sequence_number = trail.record_count;

    let record = record::new(
        stored_data,
        record_metadata,
        sequence_number,
        caller,
        timestamp,
    );

    linked_table::push_back(&mut trail.records, sequence_number, record);
    trail.record_count = trail.record_count + 1;

    event::emit(RecordAdded {
        trail_id,
        sequence_number,
        added_by: caller,
        timestamp,
    });
}

// ===== Locking =====

/// Check if a record is locked (cannot be deleted)
public fun is_record_locked<D: store + copy>(
    trail: &AuditTrail<D>,
    sequence_number: u64,
    clock: &Clock,
): bool {
    assert!(linked_table::contains(&trail.records, sequence_number), ERecordNotFound);

    let record = linked_table::borrow(&trail.records, sequence_number);
    let current_time = clock::timestamp_ms(clock);

    locking::is_locked(
        &trail.locking_config,
        sequence_number,
        record::added_at(record),
        trail.record_count,
        current_time,
    )
}

/// Update the locking configuration
///
/// TODO: Add capability parameter and permission check once implemented
public fun update_locking_config<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    new_config: LockingConfig,
    _ctx: &mut TxContext,
) {
    // TODO: check_permission(trail, cap, &permissions::locking_update(), ctx);
    trail.locking_config = new_config;
}

// ===== Metadata =====

/// Update the trail's mutable metadata
///
/// TODO: Add capability parameter and permission check once implemented
public fun update_metadata<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    new_metadata: Option<String>,
    _ctx: &mut TxContext,
) {
    // TODO: check_permission(trail, cap, &permissions::metadata_update(), ctx);
    trail.updatable_metadata = new_metadata;
}

// ===== Trail Query Functions =====

/// Get the total number of records in the trail
public fun trail_record_count<D: store + copy>(trail: &AuditTrail<D>): u64 {
    trail.record_count
}

/// Get the trail creator address
public fun trail_creator<D: store + copy>(trail: &AuditTrail<D>): address {
    trail.creator
}

/// Get the trail creation timestamp
public fun trail_created_at<D: store + copy>(trail: &AuditTrail<D>): u64 {
    trail.created_at
}

/// Get the trail's object ID
public fun trail_id<D: store + copy>(trail: &AuditTrail<D>): ID {
    object::uid_to_inner(&trail.id)
}

/// Get the trail name (immutable metadata)
public fun trail_name<D: store + copy>(trail: &AuditTrail<D>): &Option<String> {
    &trail.immutable_metadata.name
}

/// Get the trail description (immutable metadata)
public fun trail_description<D: store + copy>(trail: &AuditTrail<D>): &Option<String> {
    &trail.immutable_metadata.description
}

/// Get the updatable metadata
public fun trail_metadata<D: store + copy>(trail: &AuditTrail<D>): &Option<String> {
    &trail.updatable_metadata
}

/// Get the locking configuration
public fun trail_locking_config<D: store + copy>(trail: &AuditTrail<D>): &LockingConfig {
    &trail.locking_config
}

/// Check if the trail is empty (no records)
public fun trail_is_empty<D: store + copy>(trail: &AuditTrail<D>): bool {
    linked_table::is_empty(&trail.records)
}

/// Get the first sequence number (None if empty)
public fun trail_first_sequence<D: store + copy>(trail: &AuditTrail<D>): Option<u64> {
    *linked_table::front(&trail.records)
}

/// Get the last sequence number (None if empty)
public fun trail_last_sequence<D: store + copy>(trail: &AuditTrail<D>): Option<u64> {
    *linked_table::back(&trail.records)
}

// ===== Record Query Functions =====

/// Get a record by sequence number
public fun trail_get_record<D: store + copy>(trail: &AuditTrail<D>, sequence_number: u64): &Record<D> {
    assert!(linked_table::contains(&trail.records, sequence_number), ERecordNotFound);
    linked_table::borrow(&trail.records, sequence_number)
}

/// Check if a record exists at the given sequence number
public fun trail_has_record<D: store + copy>(trail: &AuditTrail<D>, sequence_number: u64): bool {
    linked_table::contains(&trail.records, sequence_number)
}

/// Returns all records of the audit trail
public fun trail_records<D: store + copy>(trail: &AuditTrail<D>): &LinkedTable<u64, Record<D>> {
    &trail.records
}

// ===== Role related Functions =====

/// Get the permissions associated with a specific role.
/// Aborts with ERoleDoesNotExist if the role does not exist.
public fun trail_get_role_permissions<D: store + copy>(
    trail: &AuditTrail<D>,
    role: &String,
): &VecSet<Permission> {
    assert!(vec_map::contains(&trail.roles, role), ERoleDoesNotExist);
    vec_map::get(&trail.roles, role)
}

/// Create a new role consisting of a role name and associated permissions
public fun trail_create_role<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    permissions: VecSet<Permission>,
    _ctx: &mut TxContext,
) {
    assert!(trail.has_capability_permission(cap, &permission::roles_add()), EPermissionDenied);
    vec_map::insert(&mut trail.roles, role, permissions);
}

/// Delete an existing role
public fun trail_delete_role<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role: &String,
    _ctx: &mut TxContext,
) {
    assert!(trail.has_capability_permission(cap, &permission::roles_delete()), EPermissionDenied);
    vec_map::remove(&mut trail.roles, role);
}

/// Update permissions associated with an existing role
public fun trail_update_role_permissions<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role: &String,
    new_permissions: VecSet<Permission>,
    _ctx: &mut TxContext,
) {
    assert!(trail.has_capability_permission(cap, &permission::roles_update()), EPermissionDenied);
    assert!(vec_map::contains(&trail.roles, role), ERoleDoesNotExist);
    vec_map::insert(&mut trail.roles, *role, new_permissions);
}

/// Returns the roles defined in the audit trail
public fun trail_roles<D: store + copy>(trail: &AuditTrail<D>): &VecMap<String, VecSet<Permission>> {
    &trail.roles
}

/// Indicates if the specified role exists in the audit trail
public fun trail_has_role<D: store + copy>(
    trail: &AuditTrail<D>,
    role: &String,
): bool {
    vec_map::contains(&trail.roles, role)
}


// ===== Capability related Functions =====

/// Indicates if a provided capability has a specific permission.
public fun trail_has_capability_permission<D: store + copy>(
    trail: &AuditTrail<D>,
    cap: &Capability,
    permission: &Permission,
): bool {
    assert!(trail.id() == cap.trail_id(), ETrailIdNotCorrect);
    let permissions = trail.get_role_permissions(cap.role());
    vec_set::contains(permissions, permission)
}

/// Create a new capability with a specific role
public fun trail_new_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role: &String,
    ctx: &mut TxContext,
): Capability { 
    assert!(trail.has_capability_permission(cap, &permission::capabilities_add()), EPermissionDenied);
    capability::new_capability(
        *role,
        trail.id(),
        ctx,
    )
}

/// Destroy an existing capability
/// Every owner of a capability is allowed to destroy it when no longer needed.
/// TODO: Clarify if we need to restrict access with the `CapabilitiesRevoke` permission here.
///       If yes, we also need a destroy function for Admin capabilities (without the need of another Admin capability).
///       Otherwise the last Admin capability holder will block the trail forever by not being able to destroy it.
public fun trail_destroy_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap_to_destroy: Capability,
) {
    assert!(trail.id() == cap_to_destroy.trail_id(), ETrailIdNotCorrect);
    // TODO: Implement revocation logic (e.g., remove from issued_capability set)
    cap_to_destroy.destroy();
}

public fun trail_revoke_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    cap_to_revoke: ID,
) {
    assert!(trail.has_capability_permission(cap, &permission::capabilities_revoke()), EPermissionDenied);
    // TODO: Implement revocation logic (e.g., remove from issued_capability set)
}

// ===== public use statements =====

public use fun trail_id as AuditTrail.id;
public use fun trail_creator as AuditTrail.creator;
public use fun trail_created_at as AuditTrail.created_at;
public use fun trail_add_record as AuditTrail.add_record;
public use fun trail_record_count as AuditTrail.record_count;
public use fun trail_records as AuditTrail.records;
public use fun trail_name as AuditTrail.name;
public use fun trail_description as AuditTrail.description;
public use fun trail_metadata as AuditTrail.metadata;
public use fun trail_locking_config as AuditTrail.locking_config;
public use fun trail_is_empty as AuditTrail.is_empty;
public use fun trail_first_sequence as AuditTrail.first_sequence;
public use fun trail_last_sequence as AuditTrail.last_sequence;
public use fun trail_get_record as AuditTrail.get_record;
public use fun trail_has_record as AuditTrail.has_record;
public use fun trail_has_capability_permission as AuditTrail.has_capability_permission;
public use fun trail_new_capability as AuditTrail.new_capability;
public use fun trail_destroy_capability as AuditTrail.destroy_capability;
public use fun trail_revoke_capability as AuditTrail.revoke_capability;
public use fun trail_get_role_permissions as AuditTrail.get_role_permissions;
public use fun trail_create_role as AuditTrail.create_role;
public use fun trail_delete_role as AuditTrail.delete_role;
public use fun trail_update_role_permissions as AuditTrail.update_role_permissions;
public use fun trail_roles as AuditTrail.roles;
public use fun trail_has_role as AuditTrail.has_role;