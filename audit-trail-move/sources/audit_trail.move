// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Audit Trails with role-based access control and timelock
/// A trail is a tamper-proof, sequential chain of notarized records where each
/// entry references its predecessor, ensuring verifiable continuity and
/// integrity.
module audit_trails::main;

use audit_trails::{
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
use iota::{clock::{Self, Clock}, event, linked_table::{Self, LinkedTable}, vec_set::{Self, VecSet}};
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
#[error]
const ERecordAlreadyReplaced: vector<u8> = b"The record has already been replaced";

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

/// Emitted when a trail is migrated to the current package version
public struct AuditTrailMigrated has copy, drop {
    trail_id: ID,
    migrated_by: address,
    timestamp: u64,
}

/// Emitted when mutable trail metadata is updated
public struct MetadataUpdated has copy, drop {
    trail_id: ID,
    updated_by: address,
    timestamp: u64,
}

/// Emitted when the trail's locking configuration is updated
public struct LockingConfigUpdated has copy, drop {
    trail_id: ID,
    updated_by: address,
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

/// Emitted when a record tag is added to the trail's registry
public struct RecordTagAdded has copy, drop {
    trail_id: ID,
    added_by: address,
    timestamp: u64,
}

/// Emitted when a record tag is removed from the trail's registry
public struct RecordTagRemoved has copy, drop {
    trail_id: ID,
    removed_by: address,
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
/// the permission set defined by the `permission::admin_permissions()` function. The
/// creator receives an initial admin capability that may be used to define further
/// roles and to issue capabilities to other users.
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
///
/// Emits an `AuditTrailMigrated` event on success.
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
    let trail_id = self.id();
    let timestamp = clock::timestamp_ms(clock);
    self.version = PACKAGE_VERSION;

    event::emit(AuditTrailMigrated {
        trail_id,
        migrated_by: ctx.sender(),
        timestamp,
    });
}

fun emit_locking_config_updated(trail_id: ID, updated_by: address, timestamp: u64) {
    event::emit(LockingConfigUpdated {
        trail_id,
        updated_by,
        timestamp,
    });
}

fun is_record_tag_allowed<D: store + copy>(
    self: &AuditTrail<D>,
    cap: &Capability,
    tag: &Option<String>,
): bool {
    if (tag.is_none()) {
        return true
    };

    let requested_tag = option::borrow(tag);
    assert!(record_tags::contains(&self.tags, requested_tag), ERecordTagNotDefined);
    record_tags::role_allows(&self.roles, cap, requested_tag)
}

fun remove_record<D: store + copy + drop>(
    self: &mut AuditTrail<D>,
    sequence_number: u64,
    deleted_by: address,
    timestamp: u64,
    trail_id: ID,
) {
    let record = linked_table::remove(&mut self.records, sequence_number);

    if (record.tag().is_some()) {
        record_tags::decrement_usage_count(&mut self.tags, option::borrow(record.tag()));
    };

    record.destroy();

    event::emit(RecordDeleted {
        trail_id,
        sequence_number,
        deleted_by,
        timestamp,
    });
}

/// Returns the lowest sequence_number within the last `count` records,
/// given that sequence_numbers decrease monotonically, walking from
/// the tail toward the head. Returns 0 if the table is empty or `count` is 0.
fun get_lowest_sequence_number_in_count_window<D: store + copy>(
    records: &LinkedTable<u64, Record<D>>,
    count: u64,
): u64 {
    if (count == 0) {
        return 0
    };

    let mut current = *linked_table::back(records);
    let mut remaining = count - 1;
    let mut lowest = 0;

    while (current.is_some()) {
        let current_sequence_number = current.destroy_some();
        lowest = current_sequence_number;

        if (remaining == 0) {
            break
        };

        current = *linked_table::prev(records, current_sequence_number);
        remaining = remaining - 1;
    };

    lowest
}

/// Precomputes the count-window threshold for `lock_window`.
///
/// Returns `Some(lowest_sequence_number_in_window)` when `lock_window` is a
/// count-based window with a positive count, or `None` otherwise. A record
/// with `sequence_number >= threshold` is count-locked.
fun compute_count_lock_threshold<D: store + copy>(
    records: &LinkedTable<u64, Record<D>>,
    lock_window: &LockingWindow,
): Option<u64> {
    let count_opt = lock_window.count_window();
    if (count_opt.is_some() && *count_opt.borrow() > 0) {
        option::some(get_lowest_sequence_number_in_count_window(records, count_opt.destroy_some()))
    } else {
        option::none()
    }
}

// Returns true if the record at `sequence_number` is locked by the
// `lock_window`. Uses the precomputed `count_lock_threshold` to evaluate
// count based windows and the `current_time` values to evaluate time
// based windows.
//
// Aborts if `sequence_number` is not in `records`.
fun is_record_locked_in_window<D: store + copy>(
    records: &LinkedTable<u64, Record<D>>,
    sequence_number: u64,
    lock_window: &LockingWindow,
    count_lock_threshold: &Option<u64>,
    current_time: u64,
): bool {
    // This is the shared lock-evaluation core used by `is_record_locked` and
    // `delete_records_batch`. Add new lock kinds here so both call sites pick
    // them up automatically.
    if (count_lock_threshold.is_some() && sequence_number >= *count_lock_threshold.borrow()) {
        return true
    };

    let record = records.borrow(sequence_number);
    lock_window.is_time_locked(record.added_at(), current_time)
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
    assert!(is_record_tag_allowed(self, cap, &record_tag), ERecordTagNotAllowed);

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

/// Adds a correction record that supersedes an existing record.
///
/// The original record remains immutable. The new correction record is appended
/// at the next sequence number with a correction tracker whose `replaces` set
/// contains `sequence_number`. The replaced record receives an `is_replaced_by`
/// back-pointer to the new correction so clients can resolve the current
/// canonical record by following the replacement chain.
///
/// Requires a capability granting the `CorrectRecord` permission. When either
/// the replaced record or the new correction record carries a tag, that same
/// capability must also allow the corresponding tag.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
/// * `ETrailWriteLocked` while `write_lock` is active.
/// * `ERecordNotFound` when no record exists at `sequence_number`.
/// * `ERecordAlreadyReplaced` when `sequence_number` already points to a newer
///   correction.
/// * `ERecordTagNotDefined` when `record_tag` is not in the trail's tag registry.
/// * `ERecordTagNotAllowed` when `cap`'s role does not allow the replaced
///   record tag or the new correction tag.
///
/// Emits a `RecordAdded` event for the correction record on success.
///
/// Returns the same receipt that is emitted as the `RecordAdded` event.
public fun correct_record<D: store + copy>(
    self: &mut AuditTrail<D>,
    cap: &Capability,
    sequence_number: u64,
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
            &permission::correct_record(),
            clock,
            ctx,
        );
    assert!(!locking::is_write_locked(&self.locking_config, clock), ETrailWriteLocked);
    assert!(linked_table::contains(&self.records, sequence_number), ERecordNotFound);
    assert!(
        !record::is_replaced(record::correction(self.records.borrow(sequence_number))),
        ERecordAlreadyReplaced,
    );
    assert!(
        is_record_tag_allowed(
            self,
            cap,
            self.records.borrow(sequence_number).tag(),
        ),
        ERecordTagNotAllowed,
    );
    assert!(is_record_tag_allowed(self, cap, &record_tag), ERecordTagNotAllowed);

    let caller = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = self.id();
    let seq = self.sequence_number;

    if (record_tag.is_some()) {
        record_tags::increment_usage_count(&mut self.tags, option::borrow(&record_tag));
    };

    let mut replaces = vec_set::empty();
    replaces.insert(sequence_number);

    let correction = record::new(
        stored_data,
        record_metadata,
        record_tag,
        seq,
        caller,
        timestamp,
        record::with_replaces(replaces),
    );

    record::set_replaced_by(
        record::correction_mut(linked_table::borrow_mut(&mut self.records, sequence_number)),
        seq,
    );

    linked_table::push_back(&mut self.records, seq, correction);
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
    assert!(
        is_record_tag_allowed(
            self,
            cap,
            self.records.borrow(sequence_number).tag(),
        ),
        ERecordTagNotAllowed,
    );
    assert!(!self.is_record_locked(sequence_number, clock), ERecordLocked);

    let caller = ctx.sender();
    let timestamp = clock::timestamp_ms(clock);
    let trail_id = self.id();

    self.remove_record(sequence_number, caller, timestamp, trail_id);
}

/// Deletes up to `limit` records from the front of the trail.
///
/// Walks the record list from the front and silently skips records still inside the
/// delete-record window or outside the capability's allowed tag set. Tag usage
/// counts are decremented for tagged records that are actually deleted.
///
/// `limit` caps the number of records actually deleted, not the number of records
/// inspected. Records at the front of the trail that are not eligible for deletion
/// are walked past without counting toward `limit`, so more than `limit` records may
/// be visited before `limit` deletions accumulate.
///
/// Requires a capability granting the `DeleteAllRecords` permission and, for every
/// tagged record actually deleted, a role whose `RoleTags` allow that tag.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
///
/// Emits one `RecordDeleted` event per deletion.
///
/// Returns the sequence numbers actually deleted, in deletion order. The returned
/// vector may be shorter than `limit` (or empty) if records are skipped or the
/// trail runs out of records before `limit` is reached.
///
/// Locking semantics
/// -----------------
/// The set of locked records is fixed at the start of the transaction:
///
/// * If a count-based `LockingWindow` is configured, the protected window is
///   the last `count` records present *when this call begins*. Records that
///   this same call deletes do not have an impact onto other records.
///   The oldest protected record in the count-based `LockingWindow` is
///   determined up front and its sequence_number is reused as delete criteria
///   for every other candidate record. Concurrent transactions that add
///   records or update the locking configuration are observed by *subsequent*
///   transactions only.
/// * Time-based locks are evaluated against the clock timestamp captured at
///   the start of the call, so a record's lock status is also stable for the
///   duration of the batch.
///
/// Equivalence with `delete_record`
/// --------------------------------
/// Running `delete_records_batch(limit)` produces the same final trail state as invoking
/// `delete_record` once for every sequence number this batch would delete,
/// as long as the locking configuration is not mutated and no new records are added
/// to the trail between the batch calls.
/// This holds because the count-window's lower bound is monotonic under deletion:
/// in-window records are locked and therefore never deleted, so deleting any
/// out-of-window record leaves the window's contents unchanged.
///
/// Caveats
/// -------
/// * **Partial progress.** The function always returns success even when
///   fewer than `limit` records are deleted. Callers that need to detect
///   "nothing left to delete" should inspect the length of the returned
///   vector — an empty vector means every front-to-back candidate was either
///   locked or tag-filtered out.
/// * **Tag filtering is silent.** Records whose tag is not in `cap`'s allowed
///   set are skipped without error. A capability with a narrow tag scope can
///   therefore make the batch appear to "stop early" while locked-and-disallowed
///   records still exist further back.
/// * **Gas and object-size limits.** The call walks the trail from the front
///   and deletes inline. Large `limit` values can exhaust the per-transaction
///   gas budget or hit object-mutation limits. Prefer lower `limit` values
///   resulting in modest batch sizes and repeat the call.
/// * **Front-to-back order is fixed.** There is no way to target specific
///   sequence numbers through this API — use `delete_record` for that.
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

    let lock_window = *self.locking_config.delete_record_window();

    // Precompute the count-window threshold once. Iteration deletes from the
    // front while the back is preserved, so the threshold stays valid.
    let count_lock_threshold = compute_count_lock_threshold(&self.records, &lock_window);

    let mut current = *self.records.front();

    while (deleted < limit && current.is_some()) {
        let sequence_number = current.destroy_some();
        current = *self.records.next(sequence_number);

        if (
            is_record_locked_in_window(
                &self.records,
                sequence_number,
                &lock_window,
                &count_lock_threshold,
                timestamp,
            )
        ) {
            continue
        };

        if (
            !is_record_tag_allowed(
                self,
                cap,
                self.records.borrow(sequence_number).tag(),
            )
        ) {
            continue
        };
        self.remove_record(sequence_number, caller, timestamp, trail_id);
        deleted_sequence_numbers.push_back(sequence_number);

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
/// Evaluates the trail's `delete_record_window` against the records currently
/// present in the trail and the current clock time. Count-based windows lock the
/// last N records currently present in linked-table order.
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

    let current_time = clock.timestamp_ms();
    let lock_window = self.locking_config.delete_record_window();
    let count_lock_threshold = compute_count_lock_threshold(&self.records, lock_window);

    is_record_locked_in_window(
        &self.records,
        sequence_number,
        lock_window,
        &count_lock_threshold,
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
///
/// Emits a `LockingConfigUpdated` event on success.
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

    emit_locking_config_updated(self.id(), ctx.sender(), clock::timestamp_ms(clock));
}

/// Replaces the trail's `delete_record_window` configuration.
///
/// Requires a capability granting the `UpdateLockingConfigForDeleteRecord` permission.
///
/// Aborts with:
/// * `EPackageVersionMismatch` when the trail is at a different package version.
/// * any error documented by `RoleMap::assert_capability_valid` when `cap` fails
///   authorization checks.
///
/// Emits a `LockingConfigUpdated` event on success.
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

    emit_locking_config_updated(self.id(), ctx.sender(), clock::timestamp_ms(clock));
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
///
/// Emits a `LockingConfigUpdated` event on success.
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

    emit_locking_config_updated(self.id(), ctx.sender(), clock::timestamp_ms(clock));
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
///
/// Emits a `LockingConfigUpdated` event on success.
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

    emit_locking_config_updated(self.id(), ctx.sender(), clock::timestamp_ms(clock));
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
///
/// Emits a `MetadataUpdated` event on success.
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

    event::emit(MetadataUpdated {
        trail_id: self.id(),
        updated_by: ctx.sender(),
        timestamp: clock::timestamp_ms(clock),
    });
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
///
/// Emits a `RecordTagAdded` event on success.
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

    event::emit(RecordTagAdded {
        trail_id: self.id(),
        added_by: ctx.sender(),
        timestamp: clock::timestamp_ms(clock),
    });
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
///
/// Emits a `RecordTagRemoved` event on success.
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

    event::emit(RecordTagRemoved {
        trail_id: self.id(),
        removed_by: ctx.sender(),
        timestamp: clock::timestamp_ms(clock),
    });
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
/// Emits a `tf_components::role_map::RoleCreated` event on success.
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
/// * `tf_components::role_map::ERoleDoesNotExist` when `role` is not defined on
///   the trail.
/// * `tf_components::role_map::EInitialAdminPermissionsInconsistent` when updating
///   the initial-admin role with `new_permissions` that does not include every
///   permission configured in the trail's role- and capability-admin permission sets.
/// * `ERecordTagNotDefined` when any tag in the new `role_tags` is not in the
///   trail's tag registry.
///
/// Emits a `tf_components::role_map::RoleUpdated` event on success.
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
/// * `tf_components::role_map::ERoleDoesNotExist` when `role` is not defined on
///   the trail.
/// * `tf_components::role_map::EInitialAdminRoleCannotBeDeleted` when targeting the
///   reserved initial-admin role.
///
/// Emits a `tf_components::role_map::RoleDeleted` event on success.
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
/// * `tf_components::role_map::ERoleDoesNotExist` when `role` is not defined on
///   the trail.
/// * `tf_components::capability::EValidityPeriodInconsistent` when `valid_from`
///   and `valid_until` are not consistent.
///
/// Emits a `tf_components::role_map::CapabilityIssued` event on success.
///
/// Returns the same receipt that is emitted as the `tf_components::role_map::CapabilityIssued` event.
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
/// * `tf_components::role_map::ECapabilityToRevokeHasAlreadyBeenRevoked` when
///   `cap_to_revoke` is already on the denylist.
/// * `tf_components::role_map::EInitialAdminCapabilityMustBeExplicitlyDestroyed`
///   when `cap_to_revoke` identifies an initial admin capability.
///
/// Emits a `tf_components::role_map::CapabilityRevoked` event on success.
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
/// * `tf_components::role_map::ECapabilityTargetKeyMismatch` when `cap_to_destroy`
///   was not issued for this trail.
/// * `tf_components::role_map::EInitialAdminCapabilityMustBeExplicitlyDestroyed`
///   when `cap_to_destroy` is an initial admin capability.
///
/// Emits a `tf_components::role_map::CapabilityDestroyed` event on success.
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
/// * `tf_components::role_map::ECapabilityTargetKeyMismatch` when `cap_to_destroy`
///   was not issued for this trail.
/// * `tf_components::role_map::ECapabilityIsNotInitialAdmin` when `cap_to_destroy`
///   is not an initial admin capability.
///
/// Emits a `tf_components::role_map::CapabilityDestroyed` event on success.
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
/// * `tf_components::role_map::ECapabilityIsNotInitialAdmin` when `cap_to_revoke`
///   does not identify an initial admin capability.
/// * `tf_components::role_map::ECapabilityToRevokeHasAlreadyBeenRevoked` when it is
///   already on the denylist.
///
/// Emits a `tf_components::role_map::CapabilityRevoked` event on success.
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
