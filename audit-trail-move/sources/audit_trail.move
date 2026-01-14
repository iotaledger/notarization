// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Audit Trails with role-based access control and timelock
/// A trail is a tamper-proof, sequential chain of notarized records where each
/// entry references its predecessor, ensuring verifiable continuity and
/// integrity.
module audit_trail::main;

use audit_trail::{
    capability::{Self, Capability},
    locking::{Self, LockingConfig, LockingWindow, set_delete_record_lock},
    permission::{Self, Permission},
    record::{Self, Record}
};
use iota::{
    clock::{Self, Clock},
    event,
    linked_table::{Self, LinkedTable},
    vec_map::{Self, VecMap},
    vec_set::{Self, VecSet}
};
use std::string::String;

// ===== Errors =====
#[error]
const ERecordNotFound: vector<u8> = b"Record not found at the given sequence number";
#[error]
const ERoleDoesNotExist: vector<u8> = b"The specified role does not exist in the `roles` map";
#[error]
const EPermissionDenied: vector<u8> =
    b"The role associated with the provided capability does not have the required permission";
#[error]
const ECapabilityHasBeenRevoked: vector<u8> =
    b"The provided capability has been revoked and is no longer valid";
#[error]
const ETrailIdNotCorrect: vector<u8> =
    b"The trail ID associated with the provided capability does not match the audit trail";
#[error]
const ERecordLocked: vector<u8> = b"The record is locked and cannot be deleted";

// ===== Constants =====
const INITIAL_ADMIN_ROLE_NAME: vector<u8> = b"Admin";

// ===== Core Structures =====

/// Metadata set at trail creation
public struct ImmutableMetadata has copy, drop, store {
    name: String,
    description: Option<String>,
}

/// A shared, tamper-evident ledger for storing sequential records with
/// role-based access control.
///
/// It maintains an ordered sequence of records, each assigned a unique
/// auto-incrementing sequence number.
/// Uses capability-based RBAC to manage access to the trail and its records.
public struct AuditTrail<D: store + copy> has key, store {
    id: UID,
    /// Address that created this trail
    creator: address,
    /// Creation timestamp in milliseconds
    created_at: u64,
    /// Total records added (also next sequence number)
    record_count: u64,
    /// LinkedTable mapping sequence numbers to records
    records: LinkedTable<u64, Record<D>>,
    /// Deletion locking rules
    locking_config: LockingConfig,
    /// Map of role names to permission sets.
    roles: VecMap<String, VecSet<Permission>>,
    /// Set at creation, cannot be changed
    immutable_metadata: Option<ImmutableMetadata>,
    /// Can be updated by holders of MetadataUpdate permission
    updatable_metadata: Option<String>,
    /// Whitelist of valid capability IDs
    issued_capabilities: VecSet<ID>,
}

// ===== Events =====

/// Emitted when a new trail is created
public struct AuditTrailCreated has copy, drop {
    trail_id: ID,
    creator: address,
    timestamp: u64,
    has_initial_record: bool,
}

/// Emitted when the audit trail is deleted
public struct AuditTrailDeleted has copy, drop {
    trail_id: ID,
    timestamp: u64,
}

/// Emitted when a record is added to the trail
public struct RecordAdded has copy, drop {
    trail_id: ID,
    sequence_number: u64,
    added_by: address,
    timestamp: u64,
}

/// Emitted when a record is deleted from the trail
public struct RecordDeleted has copy, drop {
    trail_id: ID,
    sequence_number: u64,
    deleted_by: address,
    timestamp: u64,
}

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
public fun new_trail_metadata(name: String, description: Option<String>): ImmutableMetadata {
    ImmutableMetadata { name, description }
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
/// * Capability with *Admin* role, allowing the creator to define custom
///   roles and issue capabilities to other users.
/// * Trail ID
public fun create<D: store + copy>(
    initial_data: Option<D>,
    initial_record_metadata: Option<String>,
    locking_config: LockingConfig,
    trail_metadata: Option<ImmutableMetadata>,
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
            0,
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

    let admin_cap = capability::new_capability(
        initial_admin_role_name(),
        trail_id,
        ctx,
    );
    let mut issued_capabilities = vec_set::empty<ID>();
    issued_capabilities.insert(admin_cap.id());

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
        issued_capabilities,
    };

    transfer::share_object(trail);

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
public fun add_record<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    stored_data: D,
    record_metadata: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(trail.has_capability_permission(cap, &permission::add_record()), EPermissionDenied);

    let caller = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = trail.id();
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

/// Delete a record from the trail by sequence number
///
/// The record must not be locked (based on the trail's locking configuration).
/// Requires the DeleteRecord permission.
public fun delete_record<D: store + copy + drop>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    sequence_number: u64,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(trail.has_capability_permission(cap, &permission::delete_record()), EPermissionDenied);
    assert!(linked_table::contains(&trail.records, sequence_number), ERecordNotFound);
    assert!(!trail.is_record_locked(sequence_number, clock), ERecordLocked);

    let caller = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = trail.id();

    let record = linked_table::remove(&mut trail.records, sequence_number);
    record::destroy(record);

    event::emit(RecordDeleted {
        trail_id,
        sequence_number,
        deleted_by: caller,
        timestamp,
    });
}

// ===== Locking =====

/// Check if a record is locked based on the trail's locking configuration.
/// Aborts with ERecordNotFound if the record doesn't exist.
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

/// Update the locking configuration. Requires `UpdateLockingConfig` permission.
public fun update_locking_config<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    new_config: LockingConfig,
    _: &mut TxContext,
) {
    assert!(
        trail.has_capability_permission(cap, &permission::update_locking_config()),
        EPermissionDenied,
    );
    trail.locking_config = new_config;
}

/// Update the `delete_record_lock` locking configuration
public fun update_locking_config_for_delete_record<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    new_delete_record_lock: LockingWindow,
    _: &mut TxContext,
) {
    assert!(
        trail.has_capability_permission(
            cap,
            &permission::update_locking_config_for_delete_record(),
        ),
        EPermissionDenied,
    );
    set_delete_record_lock(&mut trail.locking_config, new_delete_record_lock);
}

/// Update the trail's mutable metadata
public fun update_metadata<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    new_metadata: Option<String>,
    _: &mut TxContext,
) {
    assert!(
        trail.has_capability_permission(cap, &permission::update_metadata()),
        EPermissionDenied,
    );
    trail.updatable_metadata = new_metadata;
}

// ===== Trail Query Functions =====

/// Get the total number of records in the trail
public fun record_count<D: store + copy>(trail: &AuditTrail<D>): u64 {
    trail.record_count
}

/// Get the trail creator address
public fun creator<D: store + copy>(trail: &AuditTrail<D>): address {
    trail.creator
}

/// Get the trail creation timestamp
public fun created_at<D: store + copy>(trail: &AuditTrail<D>): u64 {
    trail.created_at
}

/// Get the trail's object ID
public fun id<D: store + copy>(trail: &AuditTrail<D>): ID {
    object::uid_to_inner(&trail.id)
}

/// Get the trail name
public fun name<D: store + copy>(trail: &AuditTrail<D>): Option<String> {
    trail.immutable_metadata.map!(|metadata| metadata.name)
}

/// Get the trail description
public fun description<D: store + copy>(trail: &AuditTrail<D>): Option<String> {
    if (trail.immutable_metadata.is_some()) {
        option::borrow(&trail.immutable_metadata).description
    } else {
        option::none()
    }
}

/// Get the updatable metadata
public fun metadata<D: store + copy>(trail: &AuditTrail<D>): &Option<String> {
    &trail.updatable_metadata
}

/// Get the locking configuration
public fun locking_config<D: store + copy>(trail: &AuditTrail<D>): &LockingConfig {
    &trail.locking_config
}

/// Check if the trail is empty
public fun is_empty<D: store + copy>(trail: &AuditTrail<D>): bool {
    linked_table::is_empty(&trail.records)
}

/// Get the first sequence number
public fun first_sequence<D: store + copy>(trail: &AuditTrail<D>): Option<u64> {
    *linked_table::front(&trail.records)
}

/// Get the last sequence number
public fun last_sequence<D: store + copy>(trail: &AuditTrail<D>): Option<u64> {
    *linked_table::back(&trail.records)
}

// ===== Record Query Functions =====

/// Get a record by sequence number
public fun get_record<D: store + copy>(trail: &AuditTrail<D>, sequence_number: u64): &Record<D> {
    assert!(linked_table::contains(&trail.records, sequence_number), ERecordNotFound);
    linked_table::borrow(&trail.records, sequence_number)
}

/// Check if a record exists at the given sequence number
public fun has_record<D: store + copy>(trail: &AuditTrail<D>, sequence_number: u64): bool {
    linked_table::contains(&trail.records, sequence_number)
}

/// Returns all records of the audit trail
public fun records<D: store + copy>(trail: &AuditTrail<D>): &LinkedTable<u64, Record<D>> {
    &trail.records
}

// ===== Role related Functions =====

/// Get the permissions associated with a specific role.
/// Aborts with ERoleDoesNotExist if the role does not exist.
public fun get_role_permissions<D: store + copy>(
    trail: &AuditTrail<D>,
    role: &String,
): &VecSet<Permission> {
    assert!(vec_map::contains(&trail.roles, role), ERoleDoesNotExist);
    vec_map::get(&trail.roles, role)
}

/// Create a new role consisting of a role name and associated permissions
public fun create_role<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    permissions: VecSet<Permission>,
    _: &mut TxContext,
) {
    assert!(trail.has_capability_permission(cap, &permission::add_roles()), EPermissionDenied);
    vec_map::insert(&mut trail.roles, role, permissions);
}

/// Delete an existing role
public fun delete_role<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role: &String,
    _: &mut TxContext,
) {
    assert!(trail.has_capability_permission(cap, &permission::delete_roles()), EPermissionDenied);
    vec_map::remove(&mut trail.roles, role);
}

/// Update permissions associated with an existing role
public fun update_role_permissions<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role: &String,
    new_permissions: VecSet<Permission>,
    _: &mut TxContext,
) {
    assert!(trail.has_capability_permission(cap, &permission::update_roles()), EPermissionDenied);
    assert!(vec_map::contains(&trail.roles, role), ERoleDoesNotExist);
    vec_map::remove(&mut trail.roles, role);
    vec_map::insert(&mut trail.roles, *role, new_permissions);
}

/// Returns the roles defined in the audit trail
public fun roles<D: store + copy>(trail: &AuditTrail<D>): &VecMap<String, VecSet<Permission>> {
    &trail.roles
}

/// Indicates if the specified role exists in the audit trail
public fun has_role<D: store + copy>(trail: &AuditTrail<D>, role: &String): bool {
    vec_map::contains(&trail.roles, role)
}

// ===== Capability related Functions =====

/// Indicates if a provided capability has a specific permission.
public fun has_capability_permission<D: store + copy>(
    trail: &AuditTrail<D>,
    cap: &Capability,
    permission: &Permission,
): bool {
    assert!(trail.id() == cap.trail_id(), ETrailIdNotCorrect);
    assert!(trail.issued_capabilities.contains(&cap.id()), ECapabilityHasBeenRevoked);
    let permissions = trail.get_role_permissions(cap.role());
    vec_set::contains(permissions, permission)
}

/// Create a new capability with a specific role
/// Aborts with ERoleDoesNotExist if the role does not exist.
public fun new_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role: &String,
    ctx: &mut TxContext,
): Capability {
    assert!(
        trail.has_capability_permission(cap, &permission::add_capabilities()),
        EPermissionDenied,
    );
    assert!(trail.roles.contains(role), ERoleDoesNotExist);
    let new_cap = capability::new_capability(
        *role,
        trail.id(),
        ctx,
    );
    trail.issued_capabilities.insert(new_cap.id());
    new_cap
}

/// Destroy an existing capability
/// Every owner of a capability is allowed to destroy it when no longer needed.
/// TODO: Clarify if we need to restrict access with the `CapabilitiesRevoke` permission here.
///       If yes, we also need a destroy function for Admin capabilities (without the need of another Admin capability).
///       Otherwise the last Admin capability holder will block the trail forever by not being able to destroy it.
public fun destroy_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap_to_destroy: Capability,
) {
    assert!(trail.id() == cap_to_destroy.trail_id(), ETrailIdNotCorrect);
    trail.issued_capabilities.remove(&cap_to_destroy.id());
    cap_to_destroy.destroy();
}

/// Revoke a capability. Requires `CapabilitiesRevoke` permission.
public fun revoke_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    cap_to_revoke: ID,
) {
    assert!(
        trail.has_capability_permission(cap, &permission::revoke_capabilities()),
        EPermissionDenied,
    );
    trail.issued_capabilities.remove(&cap_to_revoke);
}

/// Get the capabilities issued for this trail
public fun issued_capabilities<D: store + copy>(trail: &AuditTrail<D>): &VecSet<ID> {
    &trail.issued_capabilities
}
