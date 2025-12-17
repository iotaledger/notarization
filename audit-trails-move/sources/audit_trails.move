// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Audit trails with role-based access control and timelock
///
/// An audit trail is a tamper-proof, sequential chain of notarized records where each entry
/// references its predecessor, ensuring verifiable continuity and integrity.
///
/// Records are addressed by trail_id + sequence_number (Option B design).
module audit_trails::audit_trails;

use audit_trails::capabilities::{Self, Capability};
use audit_trails::permissions::{Self, Permission};
use iota::clock::{Self, Clock};
use iota::event;
use iota::linked_table::{Self, LinkedTable};
use iota::vec_map::{Self, VecMap};
use iota::vec_set::VecSet;
use std::string::String;

// ===== Errors =====
/// Provided previous sequence doesn't match the trail's last sequence
const EInvalidPreviousSequence: u64 = 1;
/// Capability lacks required permission or has been revoked
const EInsufficientPermissions: u64 = 2;
/// Capability is for a different trail
const EWrongCapability: u64 = 3;
/// Role doesn't exist in the trail's permission map
const ERoleNotFound: u64 = 4;
/// Attempting to create a role that already exists
const ERoleAlreadyExists: u64 = 5;
/// Role has no permissions (must have at least one)
const EEmptyRole: u64 = 6;
/// Same permission appears multiple times in input vector
const EDuplicatePermissions: u64 = 7;
/// Capability ID not in issued whitelist (forgery attempt)
const ECapabilityNotIssued: u64 = 8;
/// Signer doesn't match the capability's issued_to address
const EUnauthorizedSigner: u64 = 9;
/// Cannot remove the setup role (prevents self-bricking)
const ECannotRemoveSetupRole: u64 = 10;
/// Cannot revoke the last capability with CapAdmin permission
const ECannotRevokeLastAdmin: u64 = 11;
/// Record not found at the given sequence number
const ERecordNotFound: u64 = 12;

// ===== Core Structures =====

/// Controls when records can be deleted (time OR count based)
public struct LockingConfig has copy, drop, store {
    /// Records locked for N seconds after creation
    time_window_seconds: Option<u64>,
    /// Last N records are always locked
    count_window: Option<u64>,
}

/// Metadata set at trail creation (immutable)
public struct TrailImmutableMetadata has store {
    name: Option<String>,
    description: Option<String>,
}

/// A single record in the audit trail (stored in LinkedTable, no ObjectID)
public struct Record<D: store + copy> has store {
    /// Arbitrary data stored on-chain
    stored_data: D,
    /// Optional metadata for this specific record
    record_metadata: Option<String>,
    /// Position in the trail (0-indexed, never reused)
    sequence_number: u64,
    /// Who added this record
    added_by: address,
    /// When this record was added (milliseconds)
    added_at: u64,
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
    /// Role name → set of permissions
    permissions: VecMap<String, VecSet<Permission>>,
    /// Set at creation, cannot be changed
    immutable_metadata: TrailImmutableMetadata,
    /// Can be updated by holders of MetadataUpdate permission
    updatable_metadata: Option<String>,
    /// Whitelist of all issued capability IDs
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

/// Emitted when a record is added to the trail
/// Records are identified by trail_id + sequence_number
public struct RecordAdded has copy, drop {
    trail_id: ID,
    sequence_number: u64,
    added_by: address,
    timestamp: u64,
}

/// Emitted when a new role is defined
public struct RoleCreated has copy, drop {
    trail_id: ID,
    role: String,
    created_by: address,
}

/// Emitted when role permissions are modified
public struct RoleUpdated has copy, drop {
    trail_id: ID,
    role: String,
    updated_by: address,
}

/// Emitted when a role is deleted
public struct RoleRemoved has copy, drop {
    trail_id: ID,
    role: String,
    removed_by: address,
}

/// Emitted when a capability is revoked (removed from whitelist)
public struct CapabilityRevoked has copy, drop {
    trail_id: ID,
    capability_id: ID,
    revoked_by: address,
    timestamp: u64,
}

/// Emitted when a revoked capability is reinstated (re-added to whitelist)
public struct CapabilityReinstated has copy, drop {
    trail_id: ID,
    capability_id: ID,
    reinstated_by: address,
    timestamp: u64,
}

/// Emitted when a trail is deleted
public struct AuditTrailDeleted has copy, drop {}

// ===== Constructors =====

public fun new_locking_config(
    time_window_seconds: Option<u64>,
    count_window: Option<u64>,
): LockingConfig {
    LockingConfig { time_window_seconds, count_window }
}

public fun new_trail_metadata(
    name: Option<String>,
    description: Option<String>,
): TrailImmutableMetadata {
    TrailImmutableMetadata { name, description }
}

// ===== Trail Creation =====

/// Create a new audit trail with optional initial record
public fun create_audit_trail<D: store + copy>(
    initial_data: Option<D>,
    initial_record_metadata: Option<String>,
    locking_config: LockingConfig,
    trail_metadata: TrailImmutableMetadata,
    updatable_metadata: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
): ID {
    let creator = tx_context::sender(ctx);
    let timestamp = clock::timestamp_ms(clock);

    let trail_id = object::new(ctx);
    let trail_id_inner = object::uid_to_inner(&trail_id);

    let mut records = linked_table::new<u64, Record<D>>(ctx);
    let mut record_count = 0;
    let has_initial_record = initial_data.is_some();

    if (initial_data.is_some()) {
        let record = Record {
            stored_data: initial_data.destroy_some(),
            record_metadata: initial_record_metadata,
            sequence_number: 0,
            added_by: creator,
            added_at: timestamp,
        };

        linked_table::push_back(&mut records, 0, record);
        record_count = 1;

        event::emit(RecordAdded {
            trail_id: trail_id_inner,
            sequence_number: 0,
            added_by: creator,
            timestamp,
        });
    } else {
        initial_data.destroy_none();
    };

    // TODO: Initialize setup role with admin permissions (bootstrap)
    // The creator should receive a setup capability with PermissionAdmin + CapAdmin
    // to configure roles and issue capabilities to other users.
    //

    let trail = AuditTrail {
        id: trail_id,
        creator,
        created_at: timestamp,
        record_count,
        records,
        locking_config,
        permissions: vec_map::empty(),
        immutable_metadata: trail_metadata,
        updatable_metadata,
        issued_capabilities: iota::vec_set::empty(),
    };

    transfer::share_object(trail);

    event::emit(AuditTrailCreated {
        trail_id: trail_id_inner,
        creator,
        timestamp,
        has_initial_record,
    });

    trail_id_inner
}

// ===== Record Operations =====

/// Add a record to the trail
///
/// Validates capability permissions before allowing the operation.
/// Records are added sequentially. Use expected_sequence for optimistic concurrency.
public fun add_record<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    stored_data: D,
    record_metadata: Option<String>,
    expected_sequence: Option<u64>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::record_add(), ctx);

    let caller = tx_context::sender(ctx);
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = object::uid_to_inner(&trail.id);

    // Validate expected sequence for optimistic concurrency
    if (expected_sequence.is_some()) {
        let expected = *expected_sequence.borrow();
        assert!(expected == trail.record_count, EInvalidPreviousSequence);
    };

    let sequence_number = trail.record_count;

    let record = Record {
        stored_data,
        record_metadata,
        sequence_number,
        added_by: caller,
        added_at: timestamp,
    };

    linked_table::push_back(&mut trail.records, sequence_number, record);
    trail.record_count = trail.record_count + 1;

    event::emit(RecordAdded {
        trail_id,
        sequence_number,
        added_by: caller,
        timestamp,
    });
}

// ===== Role & Permission Management =====

/// Create a new role with specific permissions
public fun create_role<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role_name: String,
    permissions_vec: vector<Permission>,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::permission_admin(), ctx);
    assert!(!trail.permissions.contains(&role_name), ERoleAlreadyExists);

    validate_permissions(&permissions_vec);

    let perms = permissions::from_vec(permissions_vec);
    vec_map::insert(&mut trail.permissions, role_name, perms);

    event::emit(RoleCreated {
        trail_id: object::uid_to_inner(&trail.id),
        role: role_name,
        created_by: tx_context::sender(ctx),
    });
}

/// Update permissions for an existing role
public fun update_role<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role_name: String,
    permissions_vec: vector<Permission>,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::permission_admin(), ctx);
    assert!(trail.permissions.contains(&role_name), ERoleNotFound);

    validate_permissions(&permissions_vec);

    // TODO: Implement role update logic

    event::emit(RoleUpdated {
        trail_id: object::uid_to_inner(&trail.id),
        role: role_name,
        updated_by: tx_context::sender(ctx),
    });
}

/// Remove a role from the trail
public fun remove_role<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role_name: String,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::permission_admin(), ctx);
    assert!(trail.permissions.contains(&role_name), ERoleNotFound);

    // Protect setup role from removal to prevent self-bricking
    assert!(role_name != std::string::utf8(b"setup"), ECannotRemoveSetupRole);

    // TODO: Implement role removal logic
    // vec_map::remove(&mut trail.permissions, &role_name);

    event::emit(RoleRemoved {
        trail_id: object::uid_to_inner(&trail.id),
        role: role_name,
        removed_by: tx_context::sender(ctx),
    });
}

/// Validate permissions vector for duplicates and emptiness
fun validate_permissions(permissions_vec: &vector<Permission>) {
    assert!(!permissions_vec.is_empty(), EEmptyRole);

    let len = permissions_vec.length();
    let mut i = 0;
    while (i < len) {
        let perm = &permissions_vec[i];
        let mut j = i + 1;
        while (j < len) {
            assert!(perm != &permissions_vec[j], EDuplicatePermissions);
            j = j + 1;
        };
        i = i + 1;
    };
}

/// Internal permission check (validates cap, role, permission, and signer)
fun check_permission<D: store + copy>(
    trail: &AuditTrail<D>,
    cap: &Capability,
    _required: &Permission,
    _ctx: &TxContext,
) {
    let cap_id = capabilities::cap_id(cap);

    // Verify capability is in whitelist (not issued = revoked or forged)
    assert!(trail.issued_capabilities.contains(&cap_id), ECapabilityNotIssued);

    // TODO: Implement full permission checks once capabilities have role info
    // 1. Verify capability is for this trail
    // 2. Verify signer matches the capability holder
    // 3. Verify role exists
    // 4. Verify role has the required permission
}

/// Check if a role has CapAdmin permission
fun role_has_cap_admin<D: store + copy>(trail: &AuditTrail<D>, role: &String): bool {
    if (!trail.permissions.contains(role)) {
        return false
    };
    let role_perms = vec_map::get(&trail.permissions, role);
    permissions::has_permission(role_perms, &permissions::cap_admin())
}

// ===== Capability Management =====

/// Issue a new capability with a specific role
public fun issue_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    recipient: address,
    _clock: &Clock,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::cap_admin(), ctx);
    assert!(trail.permissions.contains(&role), ERoleNotFound);
    // TODO: Implement capability issuance logic
}

/// Revoke a capability by ID
public fun revoke_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    cap_id: ID,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::cap_admin(), ctx);
    // TODO: Implement capability revocation logic
}

/// Revoke multiple capabilities
public fun revoke_capabilities<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    cap_ids: vector<ID>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::cap_admin(), ctx);

    // TODO: Implement capability revocation logic
}

/// Reinstate a previously revoked capability
public fun reinstate_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    cap_id: ID,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::cap_admin(), ctx);

    // TODO: Implement capability reinstatement logic
}

/// Check if a capability has been revoked
public fun is_capability_revoked<D: store + copy>(trail: &AuditTrail<D>, cap_id: ID): bool {
    !trail.issued_capabilities.contains(&cap_id)
}

// ===== Metadata & Locking Management =====

public fun update_metadata<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    new_metadata: Option<String>,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::metadata_update(), ctx);
    trail.updatable_metadata = new_metadata;
}

public fun update_locking_config<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    new_config: LockingConfig,
    ctx: &mut TxContext,
) {
    check_permission(trail, cap, &permissions::locking_update(), ctx);
    trail.locking_config = new_config;
}

/// Check if a record is locked (cannot be deleted)
public fun is_record_locked<D: store + copy>(
    trail: &AuditTrail<D>,
    sequence_number: u64,
    clock: &Clock,
): bool {
    assert!(linked_table::contains(&trail.records, sequence_number), ERecordNotFound);

    let record = linked_table::borrow(&trail.records, sequence_number);
    let current_time = clock::timestamp_ms(clock);

    if (trail.locking_config.time_window_seconds.is_some()) {
        let time_window_ms = (*trail.locking_config.time_window_seconds.borrow() as u64) * 1000;
        let record_age = current_time - record.added_at;
        if (record_age < time_window_ms) return true
    };

    if (trail.locking_config.count_window.is_some()) {
        let count_window = *trail.locking_config.count_window.borrow();
        let records_after = trail.record_count - sequence_number - 1;
        if (records_after < count_window) return true
    };

    false
}

public fun destroy_and_revoke_capability<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: Capability,
) {
    let cap_id = capabilities::cap_id(&cap);
    trail.issued_capabilities.remove(&cap_id);
    capabilities::destroy_capability(cap);
}

// ===== Query Functions =====

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
public fun updatable_metadata<D: store + copy>(trail: &AuditTrail<D>): &Option<String> {
    &trail.updatable_metadata
}

/// Get the locking configuration
public fun locking_config<D: store + copy>(trail: &AuditTrail<D>): &LockingConfig {
    &trail.locking_config
}

/// Get permissions for a specific role
public fun role_permissions<D: store + copy>(
    trail: &AuditTrail<D>,
    role: &String,
): &VecSet<Permission> {
    vec_map::get(&trail.permissions, role)
}

/// Check if a role exists
public fun has_role<D: store + copy>(trail: &AuditTrail<D>, role: &String): bool {
    trail.permissions.contains(role)
}

/// Check if the trail is empty (no records)
public fun is_empty<D: store + copy>(trail: &AuditTrail<D>): bool {
    linked_table::is_empty(&trail.records)
}

/// Get the first sequence number (0 if trail has records)
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

/// Get the stored data from a record
public fun record_data<D: store + copy>(record: &Record<D>): &D {
    &record.stored_data
}

/// Get the record metadata
public fun record_metadata<D: store + copy>(record: &Record<D>): &Option<String> {
    &record.record_metadata
}

/// Get the record sequence number
public fun record_sequence_number<D: store + copy>(record: &Record<D>): u64 {
    record.sequence_number
}

/// Get who added the record
public fun record_added_by<D: store + copy>(record: &Record<D>): address {
    record.added_by
}

/// Get when the record was added
public fun record_added_at<D: store + copy>(record: &Record<D>): u64 {
    record.added_at
}
