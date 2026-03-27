// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Audit Trails with external authorization via OperationCap
///
/// A trail is a tamper-proof, sequential chain of notarized records where each
/// entry references its predecessor, ensuring verifiable continuity and
/// integrity.
///
/// Authorization is externalized — the trail stores a `trusted_source` ID pointing
/// to an authority object (e.g. AccessControllerBridge). Protected operations require
/// an `OperationCap` issued by that authority, verified by source binding.
module audit_trail::audit_trail;

use audit_trail::{
    actions,
    locking::{
        Self,
        LockingConfig,
        LockingWindow,
        set_config,
        set_delete_record_window,
        set_delete_trail_lock,
        set_write_lock
    },
    marker::AuditTrailPerm,
    record::{Self, Record},
    record_tags::{Self, TagRegistry}
};
use iota::{clock::{Self, Clock}, event, linked_table::{Self, LinkedTable}};
use std::string::String;
use tf_components::{operation_cap::{Self, OperationCap}, timelock::TimeLock};

// ===== Errors =====
#[error]
const ERecordNotFound: vector<u8> = b"Record not found at the given sequence number";
#[error]
const ERecordLocked: vector<u8> = b"The record is locked and cannot be deleted";
#[error]
const ETrailNotEmpty: vector<u8> = b"Audit trail cannot be deleted while records still exist";
#[error]
const ETrailDeleteLocked: vector<u8> = b"The audit trail is delete-locked";
#[error]
const ETrailWriteLocked: vector<u8> = b"The audit trail is write-locked";
#[error]
const EPackageVersionMismatch: vector<u8> =
    b"The package version of the trail does not match the expected version";
#[error]
const ERecordTagNotDefined: vector<u8> = b"The requested tag is not defined for this audit trail";
#[error]
const ERecordTagAlreadyDefined: vector<u8> =
    b"The requested tag is already defined for this audit trail";
#[error]
const ERecordTagInUse: vector<u8> =
    b"The requested tag cannot be removed because it is already used by an existing record or role";
#[error]
const ETrustedSourceNotSet: vector<u8> =
    b"The trail's trusted source has not been set";
#[error]
const ETargetMismatch: vector<u8> =
    b"The OperationCap does not target this trail";
#[error]
const ESourceMismatch: vector<u8> =
    b"The OperationCap was not issued by this trail's trusted source";
#[error]
const EPermissionDenied: vector<u8> =
    b"The OperationCap does not grant the required permission";
#[error]
const EHolderMismatch: vector<u8> =
    b"The OperationCap holder does not match the transaction sender";
#[error]
const ENotCreator: vector<u8> =
    b"Only the trail creator can set the trusted source";

// Package version, incremented when the package is updated
const PACKAGE_VERSION: u64 = 1;

// ===== Core Structures =====

/// Metadata set at trail creation
public struct ImmutableMetadata has copy, drop, store {
    name: String,
    description: Option<String>,
}

/// A shared, tamper-evident ledger for storing sequential records with
/// external authorization via OperationCap.
///
/// It maintains an ordered sequence of records, each assigned a unique
/// auto-incrementing sequence number.
/// Authorization is externalized — the trail trusts an authority object
/// identified by `trusted_source`.
public struct AuditTrail<D: store + copy> has key, store {
    id: UID,
    /// Address that created this trail
    creator: address,
    /// Creation timestamp in milliseconds
    created_at: u64,
    /// Monotonic counter for sequence assignment (never decrements)
    sequence_number: u64,
    /// LinkedTable mapping sequence numbers to records
    records: LinkedTable<u64, Record<D>>,
    /// Canonical list of tags that may be attached to records in this trail with their combined usage counts
    tags: TagRegistry,
    /// Deletion locking rules
    locking_config: LockingConfig,
    /// Set at creation, cannot be changed
    immutable_metadata: Option<ImmutableMetadata>,
    /// Can be updated by holders of UpdateMetadata permission
    updatable_metadata: Option<String>,
    /// Package version
    version: u64,
    /// The authority source this trail trusts.
    /// Points to the AccessControllerBridge's ID (or any other authority).
    /// None until explicitly set via set_trusted_source().
    /// When None, all protected operations fail (trail is inert).
    trusted_source: Option<ID>,
}

// ===== Events =====

/// Emitted when a new trail is created
public struct AuditTrailCreated has copy, drop {
    trail_id: ID,
    creator: address,
    timestamp: u64,
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

// ===== Constructors =====

/// Create immutable trail metadata
public fun new_trail_metadata(name: String, description: Option<String>): ImmutableMetadata {
    ImmutableMetadata { name, description }
}

// ===== Internal Authorization =====

/// Verify OperationCap against this trail's trusted source and required permission.
fun assert_authorized<D: store + copy>(
    self: &AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    required_action: u16,
    ctx: &TxContext,
) {
    assert!(self.trusted_source.is_some(), ETrustedSourceNotSet);
    assert!(operation_cap::target(cap) == self.id(), ETargetMismatch);
    assert!(operation_cap::source(cap) == *self.trusted_source.borrow(), ESourceMismatch);
    assert!(operation_cap::has_permission(cap, required_action), EPermissionDenied);
    assert!(operation_cap::holder(cap) == ctx.sender(), EHolderMismatch);
}

// ===== Trail Creation =====

/// Create a new audit trail with optional initial record
///
/// Returns the trail ID. The trail is inert until `set_trusted_source()` is called.
public fun create<D: store + copy>(
    initial_record: Option<record::InitialRecord<D>>,
    locking_config: LockingConfig,
    trail_metadata: Option<ImmutableMetadata>,
    updatable_metadata: Option<String>,
    tags: vector<String>,
    clock: &Clock,
    ctx: &mut TxContext,
): ID {
    let creator = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);

    let trail_uid = object::new(ctx);
    let trail_id = object::uid_to_inner(&trail_uid);

    let mut records = linked_table::new<u64, Record<D>>(ctx);
    let mut sequence_number = 0;

    if (initial_record.is_some()) {
        let record = record::into_record(
            initial_record.destroy_some(),
            0,
            creator,
            timestamp,
        );

        linked_table::push_back(&mut records, 0, record);
        sequence_number = 1;
    } else {
        initial_record.destroy_none();
    };

    let tags = record_tags::new_tag_registry(tags);

    let trail = AuditTrail {
        id: trail_uid,
        creator,
        created_at: timestamp,
        sequence_number,
        records,
        tags,
        locking_config,
        immutable_metadata: trail_metadata,
        updatable_metadata,
        version: PACKAGE_VERSION,
        trusted_source: option::none(),
    };

    transfer::share_object(trail);

    event::emit(AuditTrailCreated {
        trail_id,
        creator,
        timestamp,
    });

    trail_id
}

/// Set the trusted authority source for this trail.
/// Only callable by the trail creator.
///
/// After this, the trail only accepts OperationCaps whose `source` matches
/// the provided ID.
public fun set_trusted_source<D: store + copy>(
    self: &mut AuditTrail<D>,
    trusted_source: ID,
    ctx: &TxContext,
) {
    assert!(ctx.sender() == self.creator, ENotCreator);
    self.trusted_source = option::some(trusted_source);
}

/// Migrate the trail to the latest package version
public fun migrate<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    ctx: &TxContext,
) {
    assert!(self.version < PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::migrate(), ctx);
    self.version = PACKAGE_VERSION;
}

// ===== Record Operations =====

/// Add a record to the trail
///
/// Records are added sequentially with auto-assigned sequence numbers.
public fun add_record<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    stored_data: D,
    record_metadata: Option<String>,
    record_tag: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::add_record(), ctx);
    assert!(!locking::is_write_locked(&self.locking_config, clock), ETrailWriteLocked);

    if (record_tag.is_some()) {
        assert!(record_tags::contains(&self.tags, option::borrow(&record_tag)), ERecordTagNotDefined);
    };

    let caller = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = self.id();
    let seq = self.sequence_number;

    if (record_tag.is_some()) {
        record_tags::increment_usage_count(&mut self.tags, option::borrow(&record_tag));
    };

    let record = record::new(
        stored_data,
        record_metadata,
        record_tag,
        seq,
        caller,
        timestamp,
        record::empty(),
    );

    linked_table::push_back(&mut self.records, seq, record);
    self.sequence_number = self.sequence_number + 1;

    event::emit(RecordAdded {
        trail_id,
        sequence_number: seq,
        added_by: caller,
        timestamp,
    });
}

/// Delete a record from the trail by sequence number
///
/// The record must not be locked (based on the trail's locking configuration).
public fun delete_record<D: store + copy + drop>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    sequence_number: u64,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::delete_record(), ctx);
    assert!(linked_table::contains(&self.records, sequence_number), ERecordNotFound);
    assert!(!self.is_record_locked(sequence_number, clock), ERecordLocked);

    let caller = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = self.id();

    let record = linked_table::remove(&mut self.records, sequence_number);
    if (record::tag(&record).is_some()) {
        record_tags::decrement_usage_count(&mut self.tags, option::borrow(record::tag(&record)));
    };
    record::destroy(record);

    event::emit(RecordDeleted {
        trail_id,
        sequence_number,
        deleted_by: caller,
        timestamp,
    });
}

/// Delete up to `limit` records from the front of the trail.
///
/// This operation bypasses record locks.
/// Returns the number of records deleted in this batch.
public fun delete_records_batch<D: store + copy + drop>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    limit: u64,
    clock: &Clock,
    ctx: &mut TxContext,
): u64 {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::delete_all_records(), ctx);

    let mut deleted = 0;
    let caller = ctx.sender();
    let timestamp = clock.timestamp_ms();
    let trail_id = self.id();

    while (deleted < limit && !self.records.is_empty()) {
        let (sequence_number, record) = self.records.pop_front();

        if (record::tag(&record).is_some()) {
            record_tags::decrement_usage_count(
                &mut self.tags,
                option::borrow(record::tag(&record)),
            );
        };

        record.destroy();

        event::emit(RecordDeleted {
            trail_id,
            sequence_number,
            deleted_by: caller,
            timestamp,
        });

        deleted = deleted + 1;
    };

    deleted
}

/// Delete an empty audit trail.
///
/// Aborts if records still exist.
public fun delete_audit_trail<D: store + copy>(
    self: AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(&self, cap, actions::delete_audit_trail(), ctx);
    assert!(!locking::is_delete_trail_locked(&self.locking_config, clock), ETrailDeleteLocked);
    assert!(linked_table::is_empty(&self.records), ETrailNotEmpty);

    let trail_id = self.id();
    let timestamp = clock::timestamp_ms(clock);

    let AuditTrail {
        id,
        creator: _,
        created_at: _,
        sequence_number: _,
        records,
        tags,
        locking_config: _,
        immutable_metadata: _,
        updatable_metadata: _,
        version: _,
        trusted_source: _,
    } = self;

    linked_table::destroy_empty(records);
    tags.destroy();

    object::delete(id);

    event::emit(AuditTrailDeleted { trail_id, timestamp });
}

// ===== Locking =====

/// Check if a record is locked based on the trail's locking configuration.
/// Aborts with ERecordNotFound if the record doesn't exist.
public fun is_record_locked<D: store + copy>(
    self: &AuditTrail<D>,
    sequence_number: u64,
    clock: &Clock,
): bool {
    assert!(linked_table::contains(&self.records, sequence_number), ERecordNotFound);

    let record = linked_table::borrow(&self.records, sequence_number);
    let current_time = clock::timestamp_ms(clock);

    locking::is_delete_record_locked(
        &self.locking_config,
        sequence_number,
        record::added_at(record),
        self.sequence_number,
        current_time,
    )
}

/// Update the locking configuration.
public fun update_locking_config<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    new_config: LockingConfig,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::update_locking_config(), ctx);
    set_config(&mut self.locking_config, new_config);
}

/// Update the `delete_record_lock` locking configuration
public fun update_delete_record_window<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    new_delete_record_lock: LockingWindow,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::update_locking_config_for_delete_record(), ctx);
    set_delete_record_window(&mut self.locking_config, new_delete_record_lock);
}

/// Update the `delete_trail_lock` locking configuration.
public fun update_delete_trail_lock<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    new_delete_trail_lock: TimeLock,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::update_locking_config_for_delete_trail(), ctx);
    set_delete_trail_lock(&mut self.locking_config, new_delete_trail_lock);
}

/// Update the `write_lock` locking configuration.
public fun update_write_lock<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    new_write_lock: TimeLock,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::update_locking_config_for_write(), ctx);
    set_write_lock(&mut self.locking_config, new_write_lock);
}

/// Update the trail's mutable metadata
public fun update_metadata<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    new_metadata: Option<String>,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::update_metadata(), ctx);
    self.updatable_metadata = new_metadata;
}

/// Adds a new record tag to the trail registry.
public fun add_record_tag<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    tag: String,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::add_record_tags(), ctx);
    assert!(!self.tags.contains(&tag), ERecordTagAlreadyDefined);
    self.tags.insert_tag(tag, 0);
}

/// Removes a record tag from the trail registry if it is not used by any record.
public fun remove_record_tag<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &OperationCap<AuditTrailPerm>,
    tag: String,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert_authorized(self, cap, actions::delete_record_tags(), ctx);
    assert!(self.tags.contains(&tag), ERecordTagNotDefined);
    assert!(!self.tags.is_in_use(&tag), ERecordTagInUse);
    self.tags.remove_tag(&tag);
}

// ===== Trail Query Functions =====

/// Get the total number of records currently in the trail
public fun record_count<D: store + copy>(self: &AuditTrail<D>): u64 {
    linked_table::length(&self.records)
}

/// Get the next sequence number (monotonic counter, never decrements)
public fun sequence_number<D: store + copy>(self: &AuditTrail<D>): u64 {
    self.sequence_number
}

/// Get the trail creator address
public fun creator<D: store + copy>(self: &AuditTrail<D>): address {
    self.creator
}

/// Get the trail creation timestamp
public fun created_at<D: store + copy>(self: &AuditTrail<D>): u64 {
    self.created_at
}

/// Get the trail's object ID
public fun id<D: store + copy>(self: &AuditTrail<D>): ID {
    object::uid_to_inner(&self.id)
}

/// Get the trail name
public fun name<D: store + copy>(self: &AuditTrail<D>): Option<String> {
    self.immutable_metadata.map!(|metadata| metadata.name)
}

/// Get the trail description
public fun description<D: store + copy>(self: &AuditTrail<D>): Option<String> {
    if (self.immutable_metadata.is_some()) {
        option::borrow(&self.immutable_metadata).description
    } else {
        option::none()
    }
}

/// Get the updatable metadata
public fun metadata<D: store + copy>(self: &AuditTrail<D>): &Option<String> {
    &self.updatable_metadata
}

/// Get the locking configuration
public fun locking_config<D: store + copy>(self: &AuditTrail<D>): &LockingConfig {
    &self.locking_config
}

/// Get the trail-defined record tags and their combined usage counts.
public fun tags<D: store + copy>(self: &AuditTrail<D>): &TagRegistry {
    &self.tags
}

/// Get the trusted source ID
public fun trusted_source<D: store + copy>(self: &AuditTrail<D>): &Option<ID> {
    &self.trusted_source
}

/// Check if the trail is empty
public fun is_empty<D: store + copy>(self: &AuditTrail<D>): bool {
    linked_table::is_empty(&self.records)
}

/// Get the first sequence number
public fun first_sequence<D: store + copy>(self: &AuditTrail<D>): Option<u64> {
    *linked_table::front(&self.records)
}

/// Get the last sequence number
public fun last_sequence<D: store + copy>(self: &AuditTrail<D>): Option<u64> {
    *linked_table::back(&self.records)
}

// ===== Record Query Functions =====

/// Get a record by sequence number
public fun get_record<D: store + copy>(self: &AuditTrail<D>, sequence_number: u64): &Record<D> {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert!(linked_table::contains(&self.records, sequence_number), ERecordNotFound);
    linked_table::borrow(&self.records, sequence_number)
}

/// Check if a record exists at the given sequence number
public fun has_record<D: store + copy>(self: &AuditTrail<D>, sequence_number: u64): bool {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    linked_table::contains(&self.records, sequence_number)
}

/// Returns all records of the audit trail
public fun records<D: store + copy>(self: &AuditTrail<D>): &LinkedTable<u64, Record<D>> {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    &self.records
}
