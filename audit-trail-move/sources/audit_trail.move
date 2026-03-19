// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Audit Trails with role-based access control and timelock
/// A trail is a tamper-proof, sequential chain of notarized records where each
/// entry references its predecessor, ensuring verifiable continuity and
/// integrity.
module audit_trail::main;

use audit_trail::{
    locking::{
        Self,
        LockingConfig,
        LockingWindow,
        set_config,
        set_delete_record_window,
        set_delete_trail_lock,
        set_write_lock
    },
    permission::{Self, Permission},
    record::{Self, Record},
    record_tags::{Self, RecordTags}
};
use iota::{clock::{Self, Clock}, event, linked_table::{Self, LinkedTable}, vec_map::{Self, VecMap}, vec_set::VecSet};
use std::string::String;
use tf_components::{capability::Capability, role_map::{Self, RoleMap}, timelock::TimeLock};

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
const ERecordTagNotAllowed: vector<u8> =
    b"The provided capability cannot create records with the requested tag";
#[error]
const ERecordTagNotDefined: vector<u8> = b"The requested tag is not defined for this audit trail";
#[error]
const ERecordTagAlreadyDefined: vector<u8> =
    b"The requested tag is already defined for this audit trail";
#[error]
const ERecordTagInUse: vector<u8> =
    b"The requested tag cannot be removed because it is already used by an existing record";
// ===== Constants =====
const INITIAL_ADMIN_ROLE_NAME: vector<u8> = b"Admin";

// Package version, incremented when the package is updated
const PACKAGE_VERSION: u64 = 1;

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
    /// Monotonic counter for sequence assignment (never decrements)
    sequence_number: u64,
    /// LinkedTable mapping sequence numbers to records
    records: LinkedTable<u64, Record<D>>,
    /// Canonical list of tags that may be attached to records in this trail with their combined usage counts
    tags: VecMap<String, u64>,
    /// Deletion locking rules
    locking_config: LockingConfig,
    /// A list of role definitions consisting of a unique role specifier and a list of associated permissions
    roles: RoleMap<Permission, RecordTags>,
    /// Set at creation, cannot be changed
    immutable_metadata: Option<ImmutableMetadata>,
    /// Can be updated by holders of MetadataUpdate permission
    updatable_metadata: Option<String>,
    /// Package version
    version: u64,
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
    initial_record: Option<record::InitialRecord<D>>,
    locking_config: LockingConfig,
    trail_metadata: Option<ImmutableMetadata>,
    updatable_metadata: Option<String>,
    tags: vector<String>,
    clock: &Clock,
    ctx: &mut TxContext,
): (Capability, ID) {
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

        event::emit(RecordAdded {
            trail_id,
            sequence_number: 0,
            added_by: creator,
            timestamp,
        });
    } else {
        initial_record.destroy_none();
    };

    let role_admin_permissions = role_map::new_role_admin_permissions(
        permission::add_roles(),
        permission::delete_roles(),
        permission::update_roles(),
    );

    let capability_admin_permissions = role_map::new_capability_admin_permissions(
        permission::add_capabilities(),
        permission::revoke_capabilities(),
    );

    let (roles, admin_cap) = role_map::new(
        trail_id,
        initial_admin_role_name(),
        permission::admin_permissions(),
        role_admin_permissions,
        capability_admin_permissions,
        ctx,
    );

    let tags = record_tags::new_usage(tags);

    let trail = AuditTrail {
        id: trail_uid,
        creator,
        created_at: timestamp,
        sequence_number,
        records,
        tags,
        locking_config,
        roles,
        immutable_metadata: trail_metadata,
        updatable_metadata,
        version: PACKAGE_VERSION,
    };

    transfer::share_object(trail);

    event::emit(AuditTrailCreated {
        trail_id,
        creator,
        timestamp,
    });

    (admin_cap, trail_id)
}

public fun initial_admin_role_name(): String {
    INITIAL_ADMIN_ROLE_NAME.to_string()
}

/// Migrate the trail to the latest package version
entry fun migrate<D: store + copy>(
    trail: &mut AuditTrail<D>,
    cap: &Capability,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(trail.version < PACKAGE_VERSION, EPackageVersionMismatch);
    trail
        .roles
        .assert_capability_valid(
            cap,
            &permission::migrate_audit_trail(),
            clock,
            ctx,
        );
    trail.version = PACKAGE_VERSION;
}

fun assert_record_tag_allowed<D: store + copy>(
    self: &AuditTrail<D>,
    cap: &Capability,
    tag: &Option<String>,
) {
    if (tag.is_none()) {
        return
    };

    let requested_tag = option::borrow(tag);
    assert!(record_tags::is_defined(&self.tags, requested_tag), ERecordTagNotDefined);
    assert!(record_tags::role_allows(&self.roles, cap, requested_tag), ERecordTagNotAllowed);
}

// ===== Record Operations =====

/// Add a record to the trail
///
/// Records are added sequentially with auto-assigned sequence numbers.
public fun add_record<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    stored_data: D,
    record_metadata: Option<String>,
    record_tag: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::add_record(),
            clock,
            ctx,
        );
    assert!(!locking::is_write_locked(&self.locking_config, clock), ETrailWriteLocked);
    assert_record_tag_allowed(self, cap, &record_tag);

    let caller = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = self.id();
    let seq = self.sequence_number;

    if (record_tag.is_some()) {
        record_tags::increment_tag_usage(&mut self.tags, option::borrow(&record_tag));
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
/// Requires the DeleteRecord permission.
public fun delete_record<D: store + copy + drop>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    sequence_number: u64,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::delete_record(),
            clock,
            ctx,
        );
    assert!(linked_table::contains(&self.records, sequence_number), ERecordNotFound);
    assert!(!self.is_record_locked(sequence_number, clock), ERecordLocked);

    let caller = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = self.id();

    let record = linked_table::remove(&mut self.records, sequence_number);
    if (record::tag(&record).is_some()) {
        record_tags::decrement_tag_usage(&mut self.tags, option::borrow(record::tag(&record)));
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
/// Requires `DeleteAllRecords` permission. This operation bypasses record locks.
/// Returns the number of records deleted in this batch.
public fun delete_records_batch<D: store + copy + drop>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    limit: u64,
    clock: &Clock,
    ctx: &mut TxContext,
): u64 {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::delete_all_records(),
            clock,
            ctx,
        );

    let mut deleted = 0;
    let caller = ctx.sender();
    let timestamp = clock.timestamp_ms();
    let trail_id = self.id();

    while (deleted < limit && !self.records.is_empty()) {
        let (sequence_number, record) = self.records.pop_front();

        if (record::tag(&record).is_some()) {
            record_tags::decrement_tag_usage(&mut self.tags, option::borrow(record::tag(&record)));
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
/// Requires `DeleteAuditTrail` permission and aborts if records still exist.
public fun delete_audit_trail<D: store + copy>(
    self: AuditTrail<D>,
    cap: &Capability,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::delete_audit_trail(),
            clock,
            ctx,
        );
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
        mut tags,
        locking_config: _,
        roles: _roles,
        immutable_metadata: _,
        updatable_metadata: _,
        version: _,
    } = self;

    linked_table::destroy_empty(records);
    while (!vec_map::is_empty(&tags)) {
        let (_, _) = vec_map::pop(&mut tags);
    };
    vec_map::destroy_empty(tags);
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

/// Update the locking configuration. Requires `UpdateLockingConfig` permission.
public fun update_locking_config<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    new_config: LockingConfig,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::update_locking_config(),
            clock,
            ctx,
        );
    set_config(&mut self.locking_config, new_config);
}

/// Update the `delete_record_lock` locking configuration
public fun update_delete_record_window<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    new_delete_record_lock: LockingWindow,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::update_locking_config_for_delete_record(),
            clock,
            ctx,
        );
    set_delete_record_window(&mut self.locking_config, new_delete_record_lock);
}

/// Update the `delete_trail_lock` locking configuration.
public fun update_delete_trail_lock<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    new_delete_trail_lock: TimeLock,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::update_locking_config_for_delete_trail(),
            clock,
            ctx,
        );
    set_delete_trail_lock(&mut self.locking_config, new_delete_trail_lock);
}

/// Update the `write_lock` locking configuration.
public fun update_write_lock<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    new_write_lock: TimeLock,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::update_locking_config_for_write(),
            clock,
            ctx,
        );
    set_write_lock(&mut self.locking_config, new_write_lock);
}

/// Update the trail's mutable metadata
public fun update_metadata<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    new_metadata: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::update_metadata(),
            clock,
            ctx,
        );
    self.updatable_metadata = new_metadata;
}

/// Adds a new record tag to the trail registry.
public fun add_record_tag<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    tag: String,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);

    self.roles.assert_capability_valid(cap, &permission::add_record_tags(), clock, ctx);

    assert!(!iota::vec_map::contains(&self.tags, &tag), ERecordTagAlreadyDefined);
    vec_map::insert(&mut self.tags, tag, 0);
}

/// Removes a record tag from the trail registry if it is not used by any record.
public fun remove_record_tag<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    tag: String,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);

    self.roles.assert_capability_valid(cap, &permission::delete_record_tags(), clock, ctx);

    assert!(iota::vec_map::contains(&self.tags, &tag), ERecordTagNotDefined);
    assert!(record_tags::usage_count(&self.tags, &tag) == 0, ERecordTagInUse);
    vec_map::remove(&mut self.tags, &tag);
}

// ===== Role and Capability Administration =====

/// Creates a new role with the provided permissions.
public fun create_role<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    permissions: VecSet<Permission>,
    record_tags: Option<RecordTags>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);

    assert!(record_tags::defined_for_trail(&self.tags, &record_tags), ERecordTagNotDefined);

    role_map::create_role(
        self.access_mut(),
        cap,
        role,
        permissions,
        copy record_tags,
        clock,
        ctx,
    );

    if (record_tags.is_some()) {
        let tags = iota::vec_set::keys(record_tags::allowed_record_tags(option::borrow(&record_tags)));
        let mut i = 0;
        let tag_count = tags.length();

        while (i < tag_count) {
            record_tags::increment_tag_usage(&mut self.tags, &tags[i]);
            i = i + 1;
        };
    };
}

/// Updates permissions for an existing role.
public fun update_role_permissions<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    new_permissions: VecSet<Permission>,
    record_tags: Option<RecordTags>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);

    assert!(record_tags::defined_for_trail(&self.tags, &record_tags), ERecordTagNotDefined);
    let old_record_tags = *role_map::get_role_data(self.access(), &role);
    role_map::update_role(
        self.access_mut(),
        cap,
        &role,
        new_permissions,
        copy record_tags,
        clock,
        ctx,
    );

    if (old_record_tags.is_some()) {
        let tags = iota::vec_set::keys(record_tags::allowed_record_tags(option::borrow(&old_record_tags)));
        let mut i = 0;
        let tag_count = tags.length();

        while (i < tag_count) {
            record_tags::decrement_tag_usage(&mut self.tags, &tags[i]);
            i = i + 1;
        };
    };

    if (record_tags.is_some()) {
        let tags = iota::vec_set::keys(record_tags::allowed_record_tags(option::borrow(&record_tags)));
        let mut i = 0;
        let tag_count = tags.length();

        while (i < tag_count) {
            record_tags::increment_tag_usage(&mut self.tags, &tags[i]);
            i = i + 1;
        };
    };
}

/// Deletes an existing role.
public fun delete_role<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    let old_record_tags = *role_map::get_role_data(self.access(), &role);
    role_map::delete_role(self.access_mut(), cap, &role, clock, ctx);

    if (old_record_tags.is_some()) {
        let tags = iota::vec_set::keys(record_tags::allowed_record_tags(option::borrow(&old_record_tags)));
        let mut i = 0;
        let tag_count = tags.length();

        while (i < tag_count) {
            record_tags::decrement_tag_usage(&mut self.tags, &tags[i]);
            i = i + 1;
        };
    };
}

/// Issues a new capability for an existing role.
///
/// The capability object is transferred to `issued_to` if provided, otherwise to the caller.
public fun new_capability<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    issued_to: Option<address>,
    valid_from: Option<u64>,
    valid_until: Option<u64>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);

    let recipient = if (issued_to.is_some()) {
        let address_ref = issued_to.borrow();
        *address_ref
    } else {
        ctx.sender()
    };

    let new_cap = role_map::new_capability(
        self.access_mut(),
        cap,
        &role,
        issued_to,
        valid_from,
        valid_until,
        clock,
        ctx,
    );
    transfer::public_transfer(new_cap, recipient);
}

/// Revokes an issued capability by ID.
public fun revoke_capability<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    capability_id: ID,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    role_map::revoke_capability(
        self.access_mut(),
        cap,
        capability_id,
        clock,
        ctx,
    );
}

/// Destroys a capability object.
///
/// Requires a capability with `RevokeCapabilities` permission.
public fun destroy_capability<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    cap_to_destroy: Capability,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::revoke_capabilities(),
            clock,
            ctx,
        );
    role_map::destroy_capability(self.access_mut(), cap_to_destroy);
}

/// Destroys an initial admin capability.
///
/// Self-service: the owner passes in their own initial admin capability to destroy it.
/// No additional authorization is required.
///
/// WARNING: If all initial admin capabilities are destroyed, the trail will be permanently
/// sealed with no admin access possible.
public fun destroy_initial_admin_capability<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap_to_destroy: Capability,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    role_map::destroy_initial_admin_capability(self.access_mut(), cap_to_destroy);
}

/// Revokes an initial admin capability by ID.
///
/// Requires a capability with `RevokeCapabilities` permission.
///
/// WARNING: If all initial admin capabilities are revoked, the trail will be permanently
/// sealed with no admin access possible.
public fun revoke_initial_admin_capability<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    capability_id: ID,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    role_map::revoke_initial_admin_capability(
        self.access_mut(),
        cap,
        capability_id,
        clock,
        ctx,
    );
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
public fun tags<D: store + copy>(self: &AuditTrail<D>): &VecMap<String, u64> {
    &self.tags
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
// ===== Access Control Functions =====

/// Returns the RoleMap managing access for the audit trail.
public fun access<D: store + copy>(self: &AuditTrail<D>): &RoleMap<Permission, RecordTags> {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    &self.roles
}

/// Returns a mutable reference to the RoleMap managing access for the audit trail.
public(package) fun access_mut<D: store + copy>(
    self: &mut AuditTrail<D>,
): &mut RoleMap<Permission, RecordTags> {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    &mut self.roles
}
