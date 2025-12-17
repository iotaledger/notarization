// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Audit trails with role-based access control and timelock
///
/// An audit trail is a tamper-proof, sequential chain of notarized records where each entry
/// references its predecessor, ensuring verifiable continuity and integrity.
///
/// Records are addressed by trail_id + sequence_number
module audit_trails::audit_trails;

use audit_trails::capabilities::{Self, Capability};
use audit_trails::locking::{Self, LockingConfig};
use audit_trails::permissions::{Self, Permission};
use audit_trails::record::{Self, Record};
use iota::clock::{Self, Clock};
use iota::event;
use iota::linked_table::{Self, LinkedTable};
use iota::vec_map::{Self, VecMap};
use iota::vec_set::VecSet;
use std::string::String;

// ===== Errors =====
#[error]
const ERecordNotFound: vector<u8> = b"Record not found at the given sequence number";

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
    /// Role name -> set of permissions (TODO: implement)
    permissions: VecMap<String, VecSet<Permission>>,
    /// Set at creation, cannot be changed
    immutable_metadata: TrailImmutableMetadata,
    /// Can be updated by holders of MetadataUpdate permission
    updatable_metadata: Option<String>,
    /// Whitelist of all issued capability IDs (TODO: implement)
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
public fun create<D: store + copy>(
    initial_data: Option<D>,
    initial_record_metadata: Option<String>,
    locking_config: LockingConfig,
    trail_metadata: TrailImmutableMetadata,
    updatable_metadata: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
): ID {
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

    // TODO: Initialize setup role with admin permissions (bootstrap)
    // The creator should receive a setup capability with PermissionAdmin + CapAdmin
    // to configure roles and issue capabilities to other users.

    let trail = AuditTrail {
        id: trail_uid,
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
        trail_id,
        creator,
        timestamp,
        has_initial_record,
    });

    trail_id
}

// ===== Record Operations =====

/// Add a record to the trail
///
/// Records are added sequentially with auto-assigned sequence numbers.
///
/// TODO: Add capability parameter and permission check once implemented
public fun add_record<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    stored_data: D,
    record_metadata: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    // TODO: check_permission(trail, cap, &permissions::record_add(), ctx);

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
public fun name<D: store + copy>(trail: &AuditTrail<D>): &Option<String> {
    &trail.immutable_metadata.name
}

/// Get the trail description (immutable metadata)
public fun description<D: store + copy>(trail: &AuditTrail<D>): &Option<String> {
    &trail.immutable_metadata.description
}

/// Get the updatable metadata
public fun metadata<D: store + copy>(trail: &AuditTrail<D>): &Option<String> {
    &trail.updatable_metadata
}

/// Get the locking configuration
public fun locking_config<D: store + copy>(trail: &AuditTrail<D>): &LockingConfig {
    &trail.locking_config
}

/// Check if the trail is empty (no records)
public fun is_empty<D: store + copy>(trail: &AuditTrail<D>): bool {
    linked_table::is_empty(&trail.records)
}

/// Get the first sequence number (None if empty)
public fun first_sequence<D: store + copy>(trail: &AuditTrail<D>): Option<u64> {
    *linked_table::front(&trail.records)
}

/// Get the last sequence number (None if empty)
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
