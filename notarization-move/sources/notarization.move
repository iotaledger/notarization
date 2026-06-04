// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Implements the core `Notarization<D>` object and the shared state, metadata
/// and locking primitives reused by the `dynamic_notarization` and
/// `locked_notarization` wrapper modules.
#[allow(lint(self_transfer))]
module iota_notarization::notarization;

use iota::{clock::{Self, Clock}, event};
use iota_notarization::{
    method::{NotarizationMethod, new_dynamic, new_locked},
    timelock::{Self, TimeLock}
};
use std::string::String;

// ===== Constants =====
/// Raised when `state` or `updatable_metadata` is updated while the update lock is active.
const EUpdateWhileLocked: u64 = 0;
/// Raised when `destroy` is called while the delete or transfer lock is active.
const EDestroyWhileLocked: u64 = 1;
/// Raised when a `LockMetadata` combination has a `delete_lock` that expires
/// before the `update_lock` or `transfer_lock`.
const ELockTimeNotSatisfied: u64 = 2;
/// Raised when a `delete_lock` is constructed as `TimeLock::UntilDestroyed`.
const EUntilDestroyedLockNotAllowed: u64 = 3;
/// Raised when the `LockMetadata` of a `Notarization` using the `Dynamic`
/// Notarization Method violates the invariants of that method (see
/// `are_dynamic_notarization_invariants_ok`).
const EDynamicNotarizationInvariants: u64 = 4;
/// Raised when the `LockMetadata` of a `Notarization` using the `Locked`
/// Notarization Method violates the invariants of that method (see
/// `are_locked_notarization_invariants_ok`).
const ELockedNotarizationInvariants: u64 = 5;

// ===== Core Type =====
/// On-chain notarization object. Stores user-defined data together with
/// immutable provenance, optional updatable metadata, and lock metadata that
/// governs whether the object can be updated, transferred, or destroyed.
/// The selected Notarization Method determines which mutations are allowed
/// after creation.
public struct Notarization<D: store + drop + copy> has key {
    id: UID,
    /// Notarized data and its associated state metadata. Mutability depends
    /// on the Notarization Method:
    /// * `Dynamic`: updatable after creation via `update_state`.
    /// * `Locked`: immutable after creation.
    state: State<D>,
    /// Provenance fixed at creation time (creation timestamp, description,
    /// optional `LockMetadata`).
    immutable_metadata: ImmutableMetadata,
    /// Free-form metadata. Mutability depends on the Notarization Method:
    /// * `Dynamic`: updatable after creation via `update_metadata`; updates
    ///   do not bump `state_version_count` nor `last_state_change_at`.
    /// * `Locked`: immutable after creation.
    ///
    /// NOTE:
    /// - `updatable_metadata` can be updated independently of `state`
    /// - Updating `updatable_metadata` does not increase the `state_version_count`
    /// - Updating `updatable_metadata` does not change the `last_state_change_at` timestamp
    /// - Use `Notarization::update_metadata()` for `updatable_metadata` updates.
    updatable_metadata: Option<String>,
    /// Timestamp of the most recent `state` change, in milliseconds since the
    /// Unix epoch.
    last_state_change_at: u64,
    /// Number of times `state` has been updated since creation.
    state_version_count: u64,
    /// Notarization Method governing the mutation and destruction rules of
    /// this `Notarization`.
    method: NotarizationMethod,
}

// ===== Metadata and Locking =====
/// Immutable provenance fields of a `Notarization`, fixed at creation time.
public struct ImmutableMetadata has store {
    /// Creation timestamp, in milliseconds since the Unix epoch.
    created_at: u64,
    /// Human-readable description of the `Notarization`.
    description: Option<String>,
    /// Optional lock metadata. Presence depends on the Notarization Method:
    /// * `Dynamic`: absent when the Dynamic-Notarization carries no transfer
    ///   lock; present otherwise.
    /// * `Locked`: always present.
    locking: Option<LockMetadata>,
}

/// Bundle of three `TimeLock`s controlling whether a `Notarization` can be
/// updated, destroyed, or transferred.
public struct LockMetadata has store {
    /// Lock guarding `update_state` and `update_metadata`.
    update_lock: TimeLock,
    /// Lock guarding `destroy`. Must not be `TimeLock::UntilDestroyed`.
    delete_lock: TimeLock,
    /// Lock guarding transfer. Its role depends on the Notarization Method:
    /// * `Dynamic`: gates `dynamic_notarization::transfer`.
    /// * `Locked`: pinned to `TimeLock::UntilDestroyed`, since
    ///   Locked-Notarizations are not transferable.
    transfer_lock: TimeLock,
}

// ===== Notarization State =====
/// Versioned state of a `Notarization`: the notarized `data` together with
/// optional state-associated `metadata`.
///
/// Whether the `State` of an existing `Notarization` may change depends on
/// the Notarization Method:
/// * `Dynamic`: `data` and `metadata` are replaced together via
///   `update_state`. Every such update bumps
///   `Notarization.state_version_count` and refreshes
///   `Notarization.last_state_change_at`, even when only `metadata` changes.
/// * `Locked`: the `State` is immutable after `Notarization` creation.
public struct State<D: store + drop + copy> has copy, drop, store {
    /// Notarized payload.
    data: D,
    /// Optional state-associated metadata, versioned together with `data`.
    metadata: Option<String>,
}

// ===== Event Types =====
/// Emitted by `update_state` after a successful state update.
public struct NotarizationUpdated<D: store + drop + copy> has copy, drop {
    /// Id of the updated `Notarization` object.
    notarization_id: ID,
    /// Value of `state_version_count` after the update.
    state_version_count: u64,
    /// The new `State` after the update.
    updated_state: State<D>,
}

/// Emitted by `destroy` after a successful destruction.
public struct NotarizationDestroyed has copy, drop {
    /// Id of the destroyed `Notarization` object.
    notarization_id: ID,
}

// ===== Constructor Functions =====
/// Constructs a `State<vector<u8>>` from raw bytes and optional metadata.
public fun new_state_from_bytes(data: vector<u8>, metadata: Option<String>): State<vector<u8>> {
    State { data, metadata }
}

/// Constructs a `State<String>` from a string payload and optional metadata.
public fun new_state_from_string(data: String, metadata: Option<String>): State<String> {
    State { data, metadata }
}

/// Constructs a `State<D>` for an arbitrary payload type and optional metadata.
public fun new_state_from_generic<D: store + drop + copy>(
    data: D,
    metadata: Option<String>,
): State<D> {
    State { data, metadata }
}

/// Constructs a `LockMetadata` from the three package-local `TimeLock`s.
///
/// Rejects combinations that would let the object be destroyed before its
/// update or transfer locks expire. When `delete_lock` is a `TimeLock::UnlockAt`,
/// its unlock time must be greater than or equal to the unlock time of any
/// `UnlockAt` `update_lock` or `transfer_lock`.
///
/// In the current implementation the legal combinations are further narrowed
/// by the method-specific invariants enforced in `new_dynamic_notarization`
/// and `new_locked_notarization`; edge cases where `delete_lock` is
/// `TimeLock::None` while other locks are `UnlockAt` are therefore not
/// reachable here.
///
/// Aborts with:
/// * `EUntilDestroyedLockNotAllowed` when `delete_lock` is `TimeLock::UntilDestroyed`.
/// * `ELockTimeNotSatisfied` when `delete_lock` is `UnlockAt` and its unlock
///   time is earlier than the unlock time of `update_lock` or `transfer_lock`.
///
/// Returns the constructed `LockMetadata`.
public fun new_lock_metadata(
    update_lock: TimeLock,
    delete_lock: TimeLock,
    transfer_lock: TimeLock,
): LockMetadata {
    assert!(!delete_lock.is_until_destroyed(), EUntilDestroyedLockNotAllowed);

    if (delete_lock.is_unlock_at()) {
        let delete_lock_time = delete_lock.get_unlock_time().destroy_some();

        if (update_lock.is_unlock_at()) {
            let update_lock_time = update_lock.get_unlock_time().destroy_some();

            assert!(delete_lock_time >= update_lock_time, ELockTimeNotSatisfied)
        };

        if (transfer_lock.is_unlock_at()) {
            let transfer_lock_time = transfer_lock.get_unlock_time().destroy_some();

            assert!(delete_lock_time >= transfer_lock_time, ELockTimeNotSatisfied)
        };
    };

    // In the current implementation the combination of locks in LockMetadata
    // is restricted by the notarization-method specific lock invariants which are guaranteed
    // by function `assert_method_specific_invariants()` and the constructor functions
    // `new_locked_notarization()` and `new_dynamic_notarization()`.
    //
    // According to these invariants we don't need to handle the edge cases where
    // delete_lock.is_none() and other locks are `TimeLock::UnlockAt`.
    //
    // These edge cases must be handled here, once new notarization-methods will
    // be added in future versions of iota_notarization, having different invariants.
    //
    // To avoid malicious or at least very surprising behavior
    // the delete_lock must always exceed all other locks (as been asserted above
    // for `delete_lock.is_unlock_at()`).
    //
    // In case delete_lock.is_none() and one of the other locks is TimeLock::UnlockAt,
    // delete_lock needs to be set to the same lock_time as the lock, having the greatest
    // lock_time.

    LockMetadata {
        update_lock,
        delete_lock,
        transfer_lock,
    }
}

/// Constructs an `ImmutableMetadata` from raw components.
///
/// This is an internal helper for the wrapper modules; callers must already
/// have validated that `locking` is well-formed for the intended notarization
/// method.
public(package) fun new_immutable_metadata(
    created_at: u64,
    description: Option<String>,
    locking: Option<LockMetadata>,
): ImmutableMetadata {
    ImmutableMetadata {
        created_at,
        description,
        locking,
    }
}

// ===== Notarization Creation Functions =====
/// Creates a new `Notarization<D>` using the `Dynamic` Notarization Method.
///
/// When `transfer_lock` is `TimeLock::None`, the resulting object has no
/// `LockMetadata` and is freely transferable. Otherwise a `LockMetadata` is
/// built with `update_lock = delete_lock = TimeLock::None` and the supplied
/// `transfer_lock`. `state_version_count` starts at `0` and
/// `last_state_change_at` is set to the current clock timestamp.
///
/// Aborts with:
/// * any error documented by `new_lock_metadata` when `transfer_lock` is not
///   `TimeLock::None`.
/// * `EDynamicNotarizationInvariants` when the resulting `ImmutableMetadata`
///   violates the invariants of the `Dynamic` Notarization Method (see
///   `are_dynamic_notarization_invariants_ok`).
///
/// Returns the constructed `Notarization<D>`.
public(package) fun new_dynamic_notarization<D: store + drop + copy>(
    state: State<D>,
    immutable_description: Option<String>,
    updatable_metadata: Option<String>,
    transfer_lock: TimeLock,
    clock: &Clock,
    ctx: &mut TxContext,
): Notarization<D> {
    let locking = if (timelock::is_none(&transfer_lock)) {
        timelock::destroy(transfer_lock, clock);
        option::none()
    } else {
        option::some(new_lock_metadata(timelock::none(), timelock::none(), transfer_lock))
    };

    let immutable_metadata = ImmutableMetadata {
        created_at: clock::timestamp_ms(clock),
        description: immutable_description,
        locking,
    };
    assert!(
        are_dynamic_notarization_invariants_ok(&immutable_metadata),
        EDynamicNotarizationInvariants,
    );

    Notarization<D> {
        id: object::new(ctx),
        state,
        immutable_metadata,
        updatable_metadata,
        last_state_change_at: clock::timestamp_ms(clock),
        state_version_count: 0,
        method: new_dynamic(),
    }
}

/// Creates a new `Notarization<D>` using the `Locked` Notarization Method.
///
/// The resulting object always carries `LockMetadata` with both `update_lock`
/// and `transfer_lock` set to `TimeLock::UntilDestroyed` and `delete_lock` set
/// to the supplied value. `state_version_count` starts at `0` and
/// `last_state_change_at` is set to the current clock timestamp.
///
/// Aborts with:
/// * any error documented by `new_lock_metadata` for the chosen `delete_lock`.
/// * `ELockedNotarizationInvariants` when the resulting `ImmutableMetadata`
///   violates the invariants of the `Locked` Notarization Method (see
///   `are_locked_notarization_invariants_ok`).
///
/// Returns the constructed `Notarization<D>`.
public(package) fun new_locked_notarization<D: store + drop + copy>(
    state: State<D>,
    immutable_description: Option<String>,
    updatable_metadata: Option<String>,
    delete_lock: TimeLock,
    clock: &Clock,
    ctx: &mut TxContext,
): Notarization<D> {
    let immutable_metadata = ImmutableMetadata {
        created_at: clock::timestamp_ms(clock),
        description: immutable_description,
        locking: option::some(
            new_lock_metadata(
                timelock::until_destroyed(),
                delete_lock,
                timelock::until_destroyed(),
            ),
        ),
    };

    assert!(
        are_locked_notarization_invariants_ok(&immutable_metadata),
        ELockedNotarizationInvariants,
    );

    Notarization<D> {
        id: object::new(ctx),
        state,
        immutable_metadata,
        updatable_metadata,
        last_state_change_at: clock::timestamp_ms(clock),
        state_version_count: 0,
        method: new_locked(),
    }
}

// ===== State Management Functions =====
/// Replaces the `state` of `self` with `new_state`.
///
/// Bumps `state_version_count` by one and refreshes `last_state_change_at` to
/// the current clock timestamp.
///
/// Behaviour depends on the Notarization Method:
/// * `Dynamic`: allways permitted - `update_lock` is allways `None`.
/// * `Locked`: always aborts, because `update_lock` is pinned to
///   `TimeLock::UntilDestroyed`.
///
/// Aborts with:
/// * `EUpdateWhileLocked` when `is_update_locked` is `true`.
///
/// Emits a `NotarizationUpdated` event on success.
public fun update_state<D: store + drop + copy>(
    self: &mut Notarization<D>,
    new_state: State<D>,
    clock: &Clock,
) {
    assert!(!self.is_update_locked(clock), EUpdateWhileLocked);

    self.state = new_state;
    self.last_state_change_at = clock::timestamp_ms(clock);
    self.state_version_count = self.state_version_count + 1;

    event::emit(NotarizationUpdated {
        notarization_id: object::uid_to_inner(&self.id),
        state_version_count: self.state_version_count,
        updated_state: new_state,
    });
}

/// Destroys `self` and releases the underlying object id.
///
/// All package-local `TimeLock`s of the optional `LockMetadata` are destroyed in
/// the process; the gating check `is_destroy_allowed` ensures that no
/// `UnlockAt` lock is still active.
///
/// Aborts with:
/// * `EDestroyWhileLocked` when `is_destroy_allowed` is `false`.
/// * `iota_notarization::timelock::ETimelockNotExpired` when any `UnlockAt`
///   lock is destroyed before it expires.
///
/// Emits a `NotarizationDestroyed` event on success.
public fun destroy<D: drop + store + copy>(self: Notarization<D>, clock: &Clock) {
    assert!(self.is_destroy_allowed(clock), EDestroyWhileLocked);

    let Notarization {
        id,
        state: _,
        immutable_metadata: ImmutableMetadata {
            created_at: _,
            description: _,
            locking,
        },
        updatable_metadata: _,
        last_state_change_at: _,
        state_version_count: _,
        method: _,
    } = self;

    if (locking.is_some()) {
        let LockMetadata { update_lock, delete_lock, transfer_lock } = option::destroy_some(
            locking,
        );

        // destroy the locks
        timelock::destroy(update_lock, clock);
        timelock::destroy(delete_lock, clock);
        timelock::destroy(transfer_lock, clock);
    } else {
        // We know Dynamic-Notarizations have no lock metadata
        option::destroy_none(locking);
    };

    let id_inner = object::uid_to_inner(&id);
    object::delete(id);
    event::emit(NotarizationDestroyed { notarization_id: id_inner });
}

/// Transfers `self` to `recipient` using the IOTA `transfer` primitive.
///
/// This helper exists only so that the wrapper modules can perform the
/// transfer without having direct access to the private fields of
/// `Notarization`; transferability checks live in the calling module.
public(package) fun transfer_notarization<D: store + drop + copy>(
    self: Notarization<D>,
    recipient: address,
) {
    transfer::transfer(self, recipient);
}

// ===== Metadata Management Functions =====
/// Replaces `updatable_metadata` with `new_metadata`.
///
/// Does not change `state`, `state_version_count`, or `last_state_change_at`.
/// The `immutable_metadata.description` field is unaffected.
///
/// Behaviour depends on the Notarization Method:
/// * `Dynamic`: permitted - `update_lock` is always `None`.
/// * `Locked`: always aborts, because `update_lock` is pinned to
///   `TimeLock::UntilDestroyed`.
///
/// Aborts with:
/// * `EUpdateWhileLocked` when `is_update_locked` is `true`.
public fun update_metadata<D: store + drop + copy>(
    self: &mut Notarization<D>,
    new_metadata: Option<String>,
    clock: &Clock,
) {
    assert!(!self.is_update_locked(clock), EUpdateWhileLocked);

    self.updatable_metadata = new_metadata;
}

// ===== Getter Functions =====
/// Returns a reference to the object id of `self`.
public fun id<D: store + drop + copy>(self: &Notarization<D>): &UID { &self.id }

/// Returns a reference to the current `State<D>` of `self`.
public fun state<D: store + drop + copy>(self: &Notarization<D>): &State<D> { &self.state }

/// Returns the creation timestamp, in milliseconds since the Unix epoch.
public fun created_at<D: store + drop + copy>(self: &Notarization<D>): u64 {
    self.immutable_metadata.created_at
}

/// Returns the timestamp of the most recent state change, in milliseconds
/// since the Unix epoch.
public fun last_change<D: store + drop + copy>(self: &Notarization<D>): u64 {
    self.last_state_change_at
}

/// Returns the number of times `state` has been updated since creation.
public fun version_count<D: store + drop + copy>(self: &Notarization<D>): u64 {
    self.state_version_count
}

/// Returns the immutable description set at creation time.
public fun description<D: store + drop + copy>(self: &Notarization<D>): Option<String> {
    self.immutable_metadata.description
}

/// Returns the current value of `updatable_metadata`.
public fun updatable_metadata<D: store + drop + copy>(self: &Notarization<D>): Option<String> {
    self.updatable_metadata
}

/// Returns the Notarization Method of `self`.
public fun notarization_method<D: store + drop + copy>(self: &Notarization<D>): NotarizationMethod {
    self.method
}

// ===== Lock-Related Getter Functions =====
/// Returns a reference to the optional `LockMetadata` of `self`.
public fun lock_metadata<D: store + drop + copy>(self: &Notarization<D>): &Option<LockMetadata> {
    &self.immutable_metadata.locking
}

/// Checks whether `self` is currently locked against `state` and
/// `updatable_metadata` updates.
///
/// The result depends on the Notarization Method:
/// * `Dynamic`: always returns `false`.
/// * `Locked`: returns whether `LockMetadata.update_lock` is currently
///   timelocked according to `clock`.
///
/// Aborts with:
/// * `EDynamicNotarizationInvariants` when the invariants of the `Dynamic`
///   Notarization Method are violated.
/// * `ELockedNotarizationInvariants` when the invariants of the `Locked`
///   Notarization Method are violated.
public fun is_update_locked<D: store + drop + copy>(self: &Notarization<D>, clock: &Clock): bool {
    assert_method_specific_invariants(self);
    if (self.method.is_dynamic()) {
        false
    } else {
        let lock_metadata = option::borrow(&self.immutable_metadata.locking);

        timelock::is_timelocked(&lock_metadata.update_lock, clock)
    }
}

/// Checks whether `self` is currently locked against destruction.
///
/// The result depends on the Notarization Method:
/// * `Dynamic`: always returns `false`.
/// * `Locked`: returns whether `LockMetadata.delete_lock` is currently
///   timelocked according to `clock`.
///
/// Aborts with:
/// * `EDynamicNotarizationInvariants` when the invariants of the `Dynamic`
///   Notarization Method are violated.
/// * `ELockedNotarizationInvariants` when the invariants of the `Locked`
///   Notarization Method are violated.
public fun is_delete_locked<D: store + drop + copy>(self: &Notarization<D>, clock: &Clock): bool {
    assert_method_specific_invariants(self);

    if (self.method.is_dynamic()) {
        false
    } else {
        let lock_metadata = option::borrow(&self.immutable_metadata.locking);

        timelock::is_timelocked(&lock_metadata.delete_lock, clock)
    }
}

/// Checks whether `self` is currently locked against transfer.
///
/// Returns `false` when `self` has no `LockMetadata`. Otherwise returns
/// whether `LockMetadata.transfer_lock` is currently timelocked according to
/// `clock`.
public fun is_transfer_locked<D: store + drop + copy>(self: &Notarization<D>, clock: &Clock): bool {
    option::is_some_and!(&self.immutable_metadata.locking, |lock_metadata| {
        timelock::is_timelocked(&lock_metadata.transfer_lock, clock)
    })
}

/// Checks whether `self` is currently eligible for destruction.
///
/// The result depends on the Notarization Method:
/// * `Dynamic`: returns `false` when an `UnlockAt`
///   `transfer_lock` has not yet expired, and `true` otherwise.
/// * `Locked`: returns `true` only when none of `update_lock`, `delete_lock`,
///   or `transfer_lock` is currently an unexpired `UnlockAt` lock.
public fun is_destroy_allowed<D: store + drop + copy>(self: &Notarization<D>, clock: &Clock): bool {
    if (self.method.is_dynamic()) {
        !option::is_some_and!(
            &self.immutable_metadata.locking,
            |lock_metadata| timelock::is_timelocked_unlock_at(
                &lock_metadata.transfer_lock,
                clock,
            ),
        )
    } else {
        let lock_metadata = option::borrow(&self.immutable_metadata.locking);

        !(
            timelock::is_timelocked_unlock_at(&lock_metadata.update_lock, clock) ||
        timelock::is_timelocked_unlock_at(&lock_metadata.delete_lock, clock) ||
        timelock::is_timelocked_unlock_at(&lock_metadata.transfer_lock, clock),
        )
    }
}

/// Asserts that the invariants on `self.immutable_metadata` required by the
/// Notarization Method of `self` hold.
///
/// The rule set is selected by `self.method`:
/// * `Dynamic`: see `are_dynamic_notarization_invariants_ok`.
/// * `Locked`: see `are_locked_notarization_invariants_ok`.
///
/// Aborts with:
/// * `EDynamicNotarizationInvariants` when `self.method` is `Dynamic` and the
///   invariants of the `Dynamic` Notarization Method are violated.
/// * `ELockedNotarizationInvariants` when `self.method` is `Locked` and the
///   invariants of the `Locked` Notarization Method are violated.
public(package) fun assert_method_specific_invariants<D: store + drop + copy>(
    self: &Notarization<D>,
) {
    if (self.method.is_dynamic()) {
        assert!(
            are_dynamic_notarization_invariants_ok(&self.immutable_metadata),
            EDynamicNotarizationInvariants,
        );
    } else if (self.method.is_locked()) {
        assert!(
            are_locked_notarization_invariants_ok(&self.immutable_metadata),
            ELockedNotarizationInvariants,
        );
    }
}

/// Checks whether `immutable_metadata` satisfies the invariants required by
/// the `Locked` Notarization Method.
///
/// These invariants require that `locking` is `option::some(_)` and that both
/// `update_lock` and `transfer_lock` are `TimeLock::UntilDestroyed`.
public(package) fun are_locked_notarization_invariants_ok(
    immutable_metadata: &ImmutableMetadata,
): bool {
    if (immutable_metadata.locking.is_some()) {
        let lock_metadata = option::borrow(&immutable_metadata.locking);
        timelock::is_until_destroyed(&lock_metadata.transfer_lock) && timelock::is_until_destroyed(&lock_metadata.update_lock)
    } else {
        false
    }
}

/// Checks whether `immutable_metadata` satisfies the invariants required by
/// the `Dynamic` Notarization Method.
///
/// These invariants permit two shapes:
/// * `locking` is `option::none()`, i.e. the Dynamic-Notarization carries no
///   transfer lock; or
/// * `locking` is `option::some(_)` with `update_lock` and `delete_lock` both
///   `TimeLock::None` and `transfer_lock` anything other than `TimeLock::None`.
public(package) fun are_dynamic_notarization_invariants_ok(
    immutable_metadata: &ImmutableMetadata,
): bool {
    if (immutable_metadata.locking.is_some()) {
        let lock_metadata = option::borrow(&immutable_metadata.locking);

        timelock::is_none(&lock_metadata.delete_lock) &&
        timelock::is_none(&lock_metadata.update_lock) &&
        !timelock::is_none(&lock_metadata.transfer_lock)
    } else {
        true
    }
}

// ===== Test-only Functions =====
#[test_only]
public(package) fun destroy_lock_metadata(lock_metadata: LockMetadata, clock: &Clock) {
    let LockMetadata {
        update_lock,
        delete_lock,
        transfer_lock,
    } = lock_metadata;

    timelock::destroy(update_lock, clock);
    timelock::destroy(delete_lock, clock);
    timelock::destroy(transfer_lock, clock);
}

#[test_only]
public(package) fun destroy_immutable_metadata(
    immutable_metadata: ImmutableMetadata,
    clock: &Clock,
) {
    let ImmutableMetadata {
        created_at: _,
        description: _,
        locking,
    } = immutable_metadata;

    if (option::is_some(&locking)) {
        let lock_metadata = option::destroy_some(locking);
        destroy_lock_metadata(lock_metadata, clock);
    } else {
        option::destroy_none(locking);
    }
}

#[test_only]
public(package) fun create_custom_notarization<D: store + drop + copy>(
    state: State<D>,
    immutable_description: Option<String>,
    updatable_metadata: Option<String>,
    lock_metadata: Option<LockMetadata>,
    method: NotarizationMethod,
    clock: &Clock,
    ctx: &mut TxContext,
): Notarization<D> {
    Notarization<D> {
        id: object::new(ctx),
        state,
        immutable_metadata: ImmutableMetadata {
            created_at: clock::timestamp_ms(clock),
            description: immutable_description,
            locking: lock_metadata,
        },
        updatable_metadata,
        last_state_change_at: clock::timestamp_ms(clock),
        state_version_count: 0,
        method: method,
    }
}
