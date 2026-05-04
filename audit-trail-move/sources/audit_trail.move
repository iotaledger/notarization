// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Audit Trail with role-based access control and timelock
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
    record::{Self, Record, InitialRecord},
    record_tags::{Self, RoleTags, TagRegistry}
};
use iota::{clock::{Self, Clock}, event, linked_table::{Self, LinkedTable}, vec_set::VecSet};
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
    b"The requested tag cannot be removed because it is already used by an existing record or role";
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
public struct AuditTrail<D: store + copy> has key {
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
    /// A list of role definitions consisting of a unique role specifier and a list of associated permissions
    roles: RoleMap<Permission, RoleTags>,
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

/// Emitted when expired revoked-capability entries are removed from the denylist
public struct RevokedCapabilitiesCleanedUp has copy, drop {
    trail_id: ID,
    cleaned_count: u64,
    cleaned_by: address,
    timestamp: u64,
}

/// Returned when a capability is issued through the audit-trail API
public struct CapabilityIssuedReceipt has copy, drop {
    target_key: ID,
    capability_id: ID,
    role: String,
    issued_to: Option<address>,
    valid_from: Option<u64>,
    valid_until: Option<u64>,
}

// ===== Constructors =====

/// Creates an `ImmutableMetadata` value to be passed to `create`.
///
/// Returns the constructed `ImmutableMetadata`.
public fun new_trail_metadata(name: String, description: Option<String>): ImmutableMetadata {
    ImmutableMetadata { name, description }
}

// ===== Trail Creation =====

/// Creates a new audit trail with an optional initial record and shares it on-chain.
///
/// Initialises the trail's role map with a single role named "Admin" associated with
/// the permissions `DeleteAuditTrail`, `AddCapabilities`, `RevokeCapabilities`,
/// `AddRoles`, `UpdateRoles` and `DeleteRoles`. The creator receives an initial admin
/// capability that may be used to define further roles and to issue capabilities to
/// other users.
///
/// When `initial_record` is provided it is stored at sequence number `0`; otherwise
/// the trail is created empty. If the initial record carries a tag, that tag must
/// already be listed in `tags` and its usage count is bumped accordingly.
///
/// Aborts with:
/// * `ERecordTagNotDefined` when `initial_record` carries a tag that is not listed
///   in `tags`.
///
/// Emits an `AuditTrailCreated` event on success.
///
/// Returns the tuple `(admin_cap, trail_id)`: the initial admin `Capability` and the
/// ID of the newly shared `AuditTrail` object.
public fun create<D: store + copy>(
    initial_record: Option<InitialRecord<D>>,
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
    let mut tags = record_tags::new_tag_registry(tags);

    let mut records = linked_table::new<u64, Record<D>>(ctx);
    let mut sequence_number = 0;

    if (initial_record.is_some()) {
        let record = record::into_record(
            initial_record.destroy_some(),
            0,
            creator,
            timestamp,
        );

        if (record::tag(&record).is_some()) {
            let initial_tag = option::borrow(record::tag(&record));
            assert!(record_tags::contains(&tags, initial_tag), ERecordTagNotDefined);
            record_tags::increment_usage_count(&mut tags, initial_tag);
        };

        linked_table::push_back(&mut records, 0, record);
        sequence_number = 1;
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

/// Returns the name reserved for the initial admin role created by `create`.
///
/// Returns the constant string `"Admin"`.
public fun initial_admin_role_name(): String {
    INITIAL_ADMIN_ROLE_NAME.to_string()
}

/// Migrates the trail's stored data layout to the current package version.
///
/// Bumps the trail's `version` field from a previous package version to
/// `PACKAGE_VERSION`. Intended to be called once after a package upgrade.
///
/// Requires a capability granting the `Migrate` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is already at `PACKAGE_VERSION`.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
entry fun migrate<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(self.version < PACKAGE_VERSION, EPackageVersionMismatch);
    self
        .roles
        .assert_capability_valid(
            cap,
            &permission::migrate_audit_trail(),
            clock,
            ctx,
        );
    self.version = PACKAGE_VERSION;
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
    assert!(record_tags::contains(&self.tags, requested_tag), ERecordTagNotDefined);
    assert!(record_tags::role_allows(&self.roles, cap, requested_tag), ERecordTagNotAllowed);
}

// ===== Record Operations =====

/// Adds a record to the trail at the next available sequence number.
///
/// Records are appended sequentially with auto-assigned sequence numbers. When
/// `record_tag` is set, the trail's tag-registry usage count for that tag is
/// incremented.
///
/// Requires a capability granting the `AddRecord` permission and, when `record_tag`
/// is set, a role whose `RoleTags` allow that tag.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ETrailWriteLocked` while `write_lock` is active.
/// * `ERecordTagNotDefined` when `record_tag` is not in the trail's tag registry.
/// * `ERecordTagNotAllowed` when `cap`'s role does not allow `record_tag`.
///
/// Emits a `RecordAdded` event on success.
///
/// Returns the same receipt that is emitted as the `RecordAdded` event.
public fun add_record<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    stored_data: D,
    record_metadata: Option<String>,
    record_tag: Option<String>,
    clock: &Clock,
    ctx: &mut TxContext,
): RecordAdded {
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

    let output = RecordAdded {
        trail_id,
        sequence_number: seq,
        added_by: caller,
        timestamp,
    };

    event::emit(copy output);
    output
}

/// Deletes the record at `sequence_number` from the trail.
///
/// When the deleted record carries a tag, the trail's tag-registry usage count for
/// that tag is decremented.
///
/// Requires a capability granting the `DeleteRecord` permission and, when the stored
/// record carries a tag, a role whose `RoleTags` allow that tag.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ERecordNotFound` when no record exists at `sequence_number`.
/// * `ERecordTagNotAllowed` when `cap`'s role does not allow the stored record's
///   tag.
/// * `ERecordLocked` while the configured delete-record window still protects the
///   record.
///
/// Emits a `RecordDeleted` event on success.
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
    assert_record_tag_allowed(
        self,
        cap,
        record::tag(linked_table::borrow(&self.records, sequence_number)),
    );
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

/// Deletes up to `limit` records from the front of the trail.
///
/// Walks the record list from the front and silently skips records still inside the
/// delete-record window. Tag-aware authorization is applied to every record actually
/// deleted, and tag usage counts are decremented for tagged records. Because of the
/// silent skipping, the returned count may be less than `limit`.
///
/// Requires a capability granting the `DeleteAllRecords` permission and, for every
/// tagged record actually deleted, a role whose `RoleTags` allow that tag.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ERecordTagNotAllowed` when an encountered record carries a tag that `cap`'s
///   role does not allow.
///
/// Emits one `RecordDeleted` event per deletion.
///
/// Returns the sequence numbers deleted in this batch, in deletion order.
public fun delete_records_batch<D: store + copy + drop>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    limit: u64,
    clock: &Clock,
    ctx: &mut TxContext,
): vector<u64> {
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
    let mut deleted_sequence_numbers = vector::empty<u64>();
    let caller = ctx.sender();
    let timestamp = clock.timestamp_ms();
    let trail_id = self.id();
    let mut current = *linked_table::front(&self.records);

    while (deleted < limit && current.is_some()) {
        let sequence_number = current.destroy_some();
        current = *linked_table::next(&self.records, sequence_number);

        if (self.is_record_locked(sequence_number, clock)) {
            continue
        };

        assert_record_tag_allowed(
            self,
            cap,
            record::tag(linked_table::borrow(&self.records, sequence_number)),
        );
        let record = linked_table::remove(&mut self.records, sequence_number);

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
        vector::push_back(&mut deleted_sequence_numbers, sequence_number);

        deleted = deleted + 1;
    };

    deleted_sequence_numbers
}

/// Deletes an empty audit trail and removes the shared object on-chain.
///
/// The trail must contain no records before it can be deleted.
///
/// Requires a capability granting the `DeleteAuditTrail` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ETrailDeleteLocked` while the configured `delete_trail_lock` is active.
/// * `ETrailNotEmpty` when records still exist.
///
/// Emits an `AuditTrailDeleted` event on success.
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
        tags,
        locking_config: _,
        roles,
        immutable_metadata: _,
        updatable_metadata: _,
        version: _,
    } = self;

    roles.destroy();
    linked_table::destroy_empty(records);
    tags.destroy();

    object::delete(id);

    event::emit(AuditTrailDeleted { trail_id, timestamp });
}

// ===== Locking =====

/// Checks whether the record at `sequence_number` is currently locked against deletion.
///
/// Evaluates the trail's `delete_record_window` against the record's metadata and the
/// current clock time.
///
/// Aborts with:
/// * `ERecordNotFound` when no record exists at `sequence_number`.
///
/// Returns `true` when the record falls inside the active delete-record window.
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

/// Replaces the trail's whole locking configuration with `new_config`.
///
/// Requires a capability granting the `UpdateLockingConfig` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `EUntilDestroyedNotSupportedForDeleteTrail` when
///   `new_config.delete_trail_lock` is `TimeLock::UntilDestroyed`.
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

/// Replaces the trail's `delete_record_window` configuration.
///
/// Requires a capability granting the `UpdateLockingConfigForDeleteRecord` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
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

/// Replaces the trail's `delete_trail_lock` timelock.
///
/// Requires a capability granting the `UpdateLockingConfigForDeleteTrail` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `EUntilDestroyedNotSupportedForDeleteTrail` when `new_delete_trail_lock` is
///   `TimeLock::UntilDestroyed`.
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

/// Replaces the trail's `write_lock` timelock.
///
/// While the new lock is active, `add_record` aborts with `ETrailWriteLocked`.
///
/// Requires a capability granting the `UpdateLockingConfigForWrite` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
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

/// Replaces or clears the trail's mutable metadata field.
///
/// Passing `option::none()` clears `updatable_metadata`.
///
/// Requires a capability granting the `UpdateMetadata` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
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

/// Adds a new record tag to the trail's tag registry.
///
/// The tag is inserted with a usage count of zero.
///
/// Requires a capability granting the `AddRecordTags` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ERecordTagAlreadyDefined` when `tag` is already in the registry.
public fun add_record_tag<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    tag: String,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);

    self.roles.assert_capability_valid(cap, &permission::add_record_tags(), clock, ctx);

    assert!(!self.tags.contains(&tag), ERecordTagAlreadyDefined);
    self.tags.insert_tag(tag, 0);
}

/// Removes a record tag from the trail's tag registry.
///
/// The tag must not currently be referenced by any record or role-tag restriction.
///
/// Requires a capability granting the `DeleteRecordTags` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ERecordTagNotDefined` when `tag` is not in the registry.
/// * `ERecordTagInUse` when it is still referenced by an existing record or
///   role-tag restriction.
public fun remove_record_tag<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    tag: String,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);

    self.roles.assert_capability_valid(cap, &permission::delete_record_tags(), clock, ctx);

    assert!(self.tags.contains(&tag), ERecordTagNotDefined);
    assert!(!self.tags.is_in_use(&tag), ERecordTagInUse);

    self.tags.remove_tag(&tag);
}

// ===== Role and Capability Administration =====

/// Creates a new role on the trail with the provided permissions and optional record-tag allowlist.
///
/// Each tag listed in `role_tags` bumps that tag's usage counter in the trail's tag
/// registry.
///
/// Requires a capability granting the `AddRoles` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ERecordTagNotDefined` when any tag listed in `role_tags` is not in the
///   trail's tag registry.
///
/// Emits a `RoleCreated` event on success.
public fun create_role<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    permissions: VecSet<Permission>,
    role_tags: Option<RoleTags>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);

    assert!(self.tags.contains_all_role_tags(&role_tags), ERecordTagNotDefined);

    role_map::create_role(
        self.access_mut(),
        cap,
        role,
        permissions,
        copy role_tags,
        clock,
        ctx,
    );

    if (role_tags.is_some()) {
        let tags = role_tags.borrow().tags().keys();
        let mut i = 0;
        let tag_count = tags.length();

        while (i < tag_count) {
            self.tags.increment_usage_count(&tags[i]);
            i = i + 1;
        };
    };
}

/// Updates the permissions and record-tag allowlist of an existing role.
///
/// Tag usage counters are adjusted to reflect the difference between the old and the
/// new role-tag sets.
///
/// Requires a capability granting the `UpdateRoles` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ERoleDoesNotExist` when `role` is not defined on the trail.
/// * `EInitialAdminPermissionsInconsistent` when updating the initial-admin role
///   with `new_permissions` that does not include every permission configured in
///   the trail's role- and capability-admin permission sets.
/// * `ERecordTagNotDefined` when any tag in the new `role_tags` is not in the
///   trail's tag registry.
///
/// Emits a `RoleUpdated` event on success.
public fun update_role_permissions<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    new_permissions: VecSet<Permission>,
    role_tags: Option<RoleTags>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);

    assert!(self.tags.contains_all_role_tags(&role_tags), ERecordTagNotDefined);
    let old_record_tags = *role_map::get_role_data(self.access(), &role);
    role_map::update_role(
        self.access_mut(),
        cap,
        &role,
        new_permissions,
        copy role_tags,
        clock,
        ctx,
    );

    if (old_record_tags.is_some()) {
        let tags = old_record_tags.borrow().tags().keys();
        let mut i = 0;
        let tag_count = tags.length();

        while (i < tag_count) {
            self.tags.decrement_usage_count(&tags[i]);
            i = i + 1;
        };
    };

    if (role_tags.is_some()) {
        let tags = role_tags.borrow().tags().keys();
        let mut i = 0;
        let tag_count = tags.length();

        while (i < tag_count) {
            self.tags.increment_usage_count(&tags[i]);
            i = i + 1;
        };
    };
}

/// Deletes an existing role from the trail.
///
/// Decrements the usage count of every tag that was referenced by the role's
/// `RoleTags`. The reserved initial-admin role (`INITIAL_ADMIN_ROLE_NAME`) cannot be
/// deleted.
///
/// Requires a capability granting the `DeleteRoles` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ERoleDoesNotExist` when `role` is not defined on the trail.
/// * `EInitialAdminRoleCannotBeDeleted` when targeting the reserved initial-admin
///   role.
///
/// Emits a `RoleDeleted` event on success.
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
        let tags = old_record_tags.borrow().tags().keys();
        let mut i = 0;
        let tag_count = tags.length();

        while (i < tag_count) {
            self.tags.decrement_usage_count(&tags[i]);
            i = i + 1;
        };
    };
}

/// Issues a new capability for an existing role and transfers it to its recipient.
///
/// The capability object is transferred to `issued_to` if provided, otherwise to the
/// caller. `valid_from` and `valid_until` (milliseconds since the Unix epoch) configure
/// usage restrictions that are enforced on-chain whenever the capability is later
/// presented for authorization.
///
/// Requires a capability granting the `AddCapabilities` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ERoleDoesNotExist` when `role` is not defined on the trail.
/// * `tf_components::capability::EValidityPeriodInconsistent` when `valid_from`
///   and `valid_until` are not consistent.
///
/// Emits a `CapabilityIssued` event on success.
///
/// Returns the same receipt that is emitted as the `CapabilityIssued` event.
public fun new_capability<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    role: String,
    issued_to: Option<address>,
    valid_from: Option<u64>,
    valid_until: Option<u64>,
    clock: &Clock,
    ctx: &mut TxContext,
): CapabilityIssuedReceipt {
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
    let output = CapabilityIssuedReceipt {
        target_key: self.id(),
        capability_id: new_cap.id(),
        role: *new_cap.role(),
        issued_to: *new_cap.issued_to(),
        valid_from: *new_cap.valid_from(),
        valid_until: *new_cap.valid_until(),
    };
    transfer::public_transfer(new_cap, recipient);
    output
}

/// Revokes an issued capability by ID.
///
/// Writes `cap_to_revoke` into the trail's revoked-capability denylist.
/// `cap_to_revoke_valid_until` should be the capability's original expiry so that
/// `cleanup_revoked_capabilities` can later prune the entry once that timestamp has
/// elapsed; pass `option::none()` (encoded as `0`) to keep the entry permanently.
///
/// The function does not verify that `cap_to_revoke` actually identifies an existing
/// capability issued by this trail — any ID will be accepted and stored. Callers are
/// expected to track issued capability IDs (and their optional expiries) off-chain:
/// the trail uses a denylist, not an allowlist, to keep storage costs low when many
/// capabilities are issued.
///
/// Initial admin capabilities cannot be revoked via this function; use
/// `revoke_initial_admin_capability` instead.
///
/// Requires a capability granting the `RevokeCapabilities` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ECapabilityToRevokeHasAlreadyBeenRevoked` when `cap_to_revoke` is already on
///   the denylist.
/// * `EInitialAdminCapabilityMustBeExplicitlyDestroyed` when `cap_to_revoke`
///   identifies an initial admin capability.
///
/// Emits a `CapabilityRevoked` event on success.
public fun revoke_capability<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    cap_to_revoke: ID,
    cap_to_revoke_valid_until: Option<u64>,
    clock: &Clock,
    ctx: &TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    role_map::revoke_capability(
        self.access_mut(),
        cap,
        cap_to_revoke,
        cap_to_revoke_valid_until,
        clock,
        ctx,
    );
}

/// Destroys a capability object and removes any matching entry from the denylist.
///
/// If `cap_to_destroy` is currently on the trail's revoked-capability denylist, its
/// entry is removed as part of the destruction. Initial admin capabilities cannot be
/// destroyed via this function; use `destroy_initial_admin_capability` instead.
///
/// Requires a capability granting the `RevokeCapabilities` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ECapabilityTargetKeyMismatch` when `cap_to_destroy` was not issued for this
///   trail.
/// * `EInitialAdminCapabilityMustBeExplicitlyDestroyed` when `cap_to_destroy` is
///   an initial admin capability.
///
/// Emits a `CapabilityDestroyed` event on success.
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

/// Destroys an initial admin capability owned by the caller.
///
/// Self-service operation: the owner passes in their own initial admin capability;
/// no additional authorization is required. If the capability is currently on the
/// trail's revoked-capability denylist, its entry is removed as part of the
/// destruction.
///
/// WARNING: If all initial admin capabilities are destroyed, the trail will be
/// permanently sealed with no admin access possible.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * `ECapabilityTargetKeyMismatch` when `cap_to_destroy` was not issued for this
///   trail.
/// * `ECapabilityIsNotInitialAdmin` when `cap_to_destroy` is not an initial admin
///   capability.
///
/// Emits a `CapabilityDestroyed` event on success.
public fun destroy_initial_admin_capability<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap_to_destroy: Capability,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    role_map::destroy_initial_admin_capability(self.access_mut(), cap_to_destroy);
}

/// Revokes an initial admin capability by ID.
///
/// See `revoke_capability` for the meaning of `cap_to_revoke` and
/// `cap_to_revoke_valid_until` and the off-chain tracking of issued capabilities the
/// trail expects.
///
/// WARNING: If all initial admin capabilities are revoked, the trail will be
/// permanently sealed with no admin access possible.
///
/// Requires a capability granting the `RevokeCapabilities` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ECapabilityIsNotInitialAdmin` when `cap_to_revoke` does not identify an
///   initial admin capability.
/// * `ECapabilityToRevokeHasAlreadyBeenRevoked` when it is already on the
///   denylist.
///
/// Emits a `CapabilityRevoked` event on success.
public fun revoke_initial_admin_capability<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    cap_to_revoke: ID,
    cap_to_revoke_valid_until: Option<u64>,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    role_map::revoke_initial_admin_capability(
        self.access_mut(),
        cap,
        cap_to_revoke,
        cap_to_revoke_valid_until,
        clock,
        ctx,
    );
}

/// Removes already-expired entries from the trail's revoked-capability denylist.
///
/// Iterates through the denylist and removes every entry whose `valid_until`
/// timestamp is non-zero and less than the current clock time. Entries with
/// `valid_until == 0` (capabilities that had no expiry) are kept since they remain
/// potentially valid and must stay on the denylist. See `revoke_capability` for the
/// rationale behind off-chain tracking of issued capabilities.
///
/// Requires a capability granting the `RevokeCapabilities` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
///
/// Emits a RevokedCapabilitiesCleanedUp event on success.
///
/// Returns the same receipt that is emitted as the `RevokedCapabilitiesCleanedUp` event.
public fun cleanup_revoked_capabilities<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    clock: &Clock,
    ctx: &TxContext,
): RevokedCapabilitiesCleanedUp {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    let revoked_count_before = linked_table::length(role_map::revoked_capabilities(self.access()));
    self
        .access_mut()
        .cleanup_revoked_capabilities(
            cap,
            clock,
            ctx,
        );
    let revoked_count_after = linked_table::length(role_map::revoked_capabilities(self.access()));
    let output = RevokedCapabilitiesCleanedUp {
        trail_id: self.id(),
        cleaned_count: revoked_count_before - revoked_count_after,
        cleaned_by: ctx.sender(),
        timestamp: clock::timestamp_ms(clock),
    };
    event::emit(copy output);
    output
}

// ===== Trail Query Functions =====

/// Returns the total number of records currently stored in the trail.
public fun record_count<D: store + copy>(self: &AuditTrail<D>): u64 {
    linked_table::length(&self.records)
}

/// Returns the next sequence number that will be assigned to a new record.
///
/// The sequence number is a monotonic counter that never decrements.
public fun sequence_number<D: store + copy>(self: &AuditTrail<D>): u64 {
    self.sequence_number
}

/// Returns the address that created this trail.
public fun creator<D: store + copy>(self: &AuditTrail<D>): address {
    self.creator
}

/// Returns the trail's creation timestamp in milliseconds since the Unix epoch.
public fun created_at<D: store + copy>(self: &AuditTrail<D>): u64 {
    self.created_at
}

/// Returns the trail's on-chain object ID.
public fun id<D: store + copy>(self: &AuditTrail<D>): ID {
    object::uid_to_inner(&self.id)
}

/// Returns the trail's immutable name from `ImmutableMetadata`, when set.
public fun name<D: store + copy>(self: &AuditTrail<D>): Option<String> {
    self.immutable_metadata.map!(|metadata| metadata.name)
}

/// Returns the trail's immutable description from `ImmutableMetadata`, when set.
public fun description<D: store + copy>(self: &AuditTrail<D>): Option<String> {
    if (self.immutable_metadata.is_some()) {
        option::borrow(&self.immutable_metadata).description
    } else {
        option::none()
    }
}

/// Returns a reference to the trail's mutable `updatable_metadata` field.
public fun metadata<D: store + copy>(self: &AuditTrail<D>): &Option<String> {
    &self.updatable_metadata
}

/// Returns a reference to the trail's `LockingConfig`.
public fun locking_config<D: store + copy>(self: &AuditTrail<D>): &LockingConfig {
    &self.locking_config
}

/// Returns a reference to the trail's record-tag registry with combined usage counts.
public fun tags<D: store + copy>(self: &AuditTrail<D>): &TagRegistry {
    &self.tags
}

/// Checks whether the trail contains any records.
///
/// Returns `true` when the trail's record list is empty.
public fun is_empty<D: store + copy>(self: &AuditTrail<D>): bool {
    linked_table::is_empty(&self.records)
}

/// Returns the sequence number of the first record in the trail, when any.
public fun first_sequence<D: store + copy>(self: &AuditTrail<D>): Option<u64> {
    *linked_table::front(&self.records)
}

/// Returns the sequence number of the last record in the trail, when any.
public fun last_sequence<D: store + copy>(self: &AuditTrail<D>): Option<u64> {
    *linked_table::back(&self.records)
}

// ===== Record Query Functions =====

/// Returns a reference to the record stored at `sequence_number`.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * `ERecordNotFound` when no record exists at `sequence_number`.
public fun get_record<D: store + copy>(self: &AuditTrail<D>, sequence_number: u64): &Record<D> {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    assert!(linked_table::contains(&self.records, sequence_number), ERecordNotFound);
    linked_table::borrow(&self.records, sequence_number)
}

/// Checks whether a record exists at the given sequence number.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
///
/// Returns `true` when a record is stored at `sequence_number`.
public fun has_record<D: store + copy>(self: &AuditTrail<D>, sequence_number: u64): bool {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    linked_table::contains(&self.records, sequence_number)
}

/// Returns a reference to the trail's record table indexed by sequence number.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
public fun records<D: store + copy>(self: &AuditTrail<D>): &LinkedTable<u64, Record<D>> {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    &self.records
}
// ===== Access Control Functions =====

/// Returns a reference to the `RoleMap` managing roles and capabilities for this trail.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
public fun access<D: store + copy>(self: &AuditTrail<D>): &RoleMap<Permission, RoleTags> {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    &self.roles
}

/// Returns a mutable reference to the `RoleMap` managing roles and capabilities for this trail.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
public(package) fun access_mut<D: store + copy>(
    self: &mut AuditTrail<D>,
): &mut RoleMap<Permission, RoleTags> {
    assert!(self.version == PACKAGE_VERSION, EPackageVersionMismatch);
    &mut self.roles
}
