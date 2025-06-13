// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides core notarization capabilities to be used by
/// locked_notarization and dynamic_notarization modules
#[allow(lint(self_transfer))]
module iota_notarization::notarization;

use iota::{clock::{Self, Clock}, event};
use iota_notarization::{
    method::{NotarizationMethod, new_dynamic, new_locked},
    timelock::{Self, TimeLock}
};
use std::string::String;

// ===== Constants =====
/// Cannot update state while notarization is locked for updates
const EUpdateWhileLocked: u64 = 0;
/// Cannot destroy while notarization is locked for deletion
const EDestroyWhileLocked: u64 = 1;
/// A lock time is not satisfied
const ELockTimeNotSatisfied: u64 = 2;
/// Delete lock cannot be TimeLock::UntilDestroyed
const EUntilDestroyedLockNotAllowed: u64 = 3;
/// Invariants for dynamic notarization are broken by the specified
/// Notarization configuration
const EDynamicNotarizationInvariants: u64 = 4;
/// Invariants for locked notarization are broken by the specified
/// Notarization configuration
const ELockedNotarizationInvariants: u64 = 5;

// ===== Core Type =====
/// A unified notarization type that can be either dynamic or locked
public struct Notarization<D: store + drop + copy> has key {
    id: UID,
    /// The state of the `Notarization` that can be updated
    state: State<D>,
    /// Variant-specific metadata
    immutable_metadata: ImmutableMetadata,
    /// Provides context or additional information for third parties
    updatable_metadata: Option<String>,
    /// Timestamp of the last state change
    last_state_change_at: u64,
    /// Counter for the number of state updates
    state_version_count: u64,
    /// Notarization Method
    method: NotarizationMethod,
}

// ===== Metadata and Locking =====
/// Gathers immutable fields defined when the `Notarization` object is created
public struct ImmutableMetadata has store {
    /// Timestamp when the `Notarization` was created
    created_at: u64,
    /// Description of the `Notarization`
    description: Option<String>,
    /// Optional lock metadata for `Notarization`
    locking: Option<LockMetadata>,
}

/// Defines how a `Notarization` is locked.
public struct LockMetadata has store {
    /// Update lock condition
    update_lock: TimeLock,
    /// Lock condition for deletion
    ///
    /// NOTE: delete_lock cannot be TimeLock::UntilDestroyed
    delete_lock: TimeLock,
    /// Transfer lock
    ///
    /// NOTE: Only dynamic notarizations can be transferable
    transfer_lock: TimeLock,
}

// ===== Notarization State =====
/// Represents the state of a Notarization that can be updated
/// Contains arbitrary data and metadata that can be updated by the owner
public struct State<D: store + drop + copy> has copy, drop, store {
    /// The data being notarized
    data: D,
    /// Mutable metadata that can be updated together with the state data
    metadata: Option<String>,
}

// ===== Event Types =====
/// Event emitted when the state of a `Notarization` is updated
public struct NotarizationUpdated<D: store + drop + copy> has copy, drop {
    /// ID of the `Notarization` object that was updated
    notarization_id: ID,
    /// New version number after the update
    state_version_count: u64,
    /// Updated State
    updated_state: State<D>,
}

/// Event emitted when a `Notarization` is destroyed
public struct NotarizationDestroyed has copy, drop {
    /// ID of the `Notarization` object that was destroyed
    notarization_id: ID,
}

// ===== Constructor Functions =====
/// Create a new state from a vector<u8> data
public fun new_state_from_bytes(data: vector<u8>, metadata: Option<String>): State<vector<u8>> {
    State { data, metadata }
}

/// Create state from a string data
public fun new_state_from_string(data: String, metadata: Option<String>): State<String> {
    State { data, metadata }
}

/// Create state from generic data
public fun new_state_from_generic<D: store + drop + copy>(
    data: D,
    metadata: Option<String>,
): State<D> {
    State { data, metadata }
}

/// Create lock metadata
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
    // is restricted by the method specific lock invariants which are guaranteed
    // by function `assert_method_specific_invariants()` and the constructor functions
    // `new_locked_notarization()` and `new_dynamic_notarization()`.
    //
    // According to these invariants we don't need to handle the edge cases where
    // delete_lock.is_none() and other locks are `TimeLock::UnlockAt`.
    //
    // These edge cases must be handled here, once new Notarization methods wil
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
/// Create a new dynamic `Notarization`
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

/// Create a new locked `Notarization`
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
/// Update the state of a `Notarization`
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

/// Destroy a `Notarization`
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
        // We know dynamic Notarizations have no lock metadata
        option::destroy_none(locking);
    };

    let id_inner = object::uid_to_inner(&id);
    object::delete(id);
    event::emit(NotarizationDestroyed { notarization_id: id_inner });
}

/// Re-exports the transfer function from the core module
///
/// Workaround for transferability
public(package) fun transfer_notarization<D: store + drop + copy>(
    self: Notarization<D>,
    recipient: address,
) {
    transfer::transfer(self, recipient);
}

// ===== Metadata Management Functions =====
/// Update the updatable metadata of a `Notarization`
/// This does not affect the state version count
public fun update_metadata<D: store + drop + copy>(
    self: &mut Notarization<D>,
    new_metadata: Option<String>,
    clock: &Clock,
) {
    assert!(!self.is_update_locked(clock), EUpdateWhileLocked);

    self.updatable_metadata = new_metadata;
}

// ===== Getter Functions =====
public fun id<D: store + drop + copy>(self: &Notarization<D>): &UID { &self.id }

public fun state<D: store + drop + copy>(self: &Notarization<D>): &State<D> { &self.state }

public fun created_at<D: store + drop + copy>(self: &Notarization<D>): u64 {
    self.immutable_metadata.created_at
}

public fun last_change<D: store + drop + copy>(self: &Notarization<D>): u64 {
    self.last_state_change_at
}

public fun version_count<D: store + drop + copy>(self: &Notarization<D>): u64 {
    self.state_version_count
}

public fun description<D: store + drop + copy>(self: &Notarization<D>): Option<String> {
    self.immutable_metadata.description
}

public fun updatable_metadata<D: store + drop + copy>(self: &Notarization<D>): Option<String> {
    self.updatable_metadata
}

public fun notarization_method<D: store + drop + copy>(self: &Notarization<D>): NotarizationMethod {
    self.method
}

// ===== Lock-Related Getter Functions =====
/// Get the lock metadata if this is a locked Notarization
public fun lock_metadata<D: store + drop + copy>(self: &Notarization<D>): &Option<LockMetadata> {
    &self.immutable_metadata.locking
}

/// Check if the `Notarization` is locked for updates (always false for dynamic variant)
public fun is_update_locked<D: store + drop + copy>(self: &Notarization<D>, clock: &Clock): bool {
    assert_method_specific_invariants(self);
    if (self.method.is_dynamic()) {
        false
    } else {
        let lock_metadata = option::borrow(&self.immutable_metadata.locking);

        timelock::is_timelocked(&lock_metadata.update_lock, clock)
    }
}

/// Check if the `Notarization` is locked for deletion (always false for dynamic variant)
public fun is_delete_locked<D: store + drop + copy>(self: &Notarization<D>, clock: &Clock): bool {
    assert_method_specific_invariants(self);

    if (self.method.is_dynamic()) {
        false
    } else {
        let lock_metadata = option::borrow(&self.immutable_metadata.locking);

        timelock::is_timelocked(&lock_metadata.delete_lock, clock)
    }
}

/// Check if the `Notarization` is locked for transfer
public fun is_transfer_locked<D: store + drop + copy>(self: &Notarization<D>, clock: &Clock): bool {
    option::is_some_and!(&self.immutable_metadata.locking, |lock_metadata| {
        timelock::is_timelocked(&lock_metadata.transfer_lock, clock)
    })
}

/// Check if the `Notarization` can be destroyed
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

/// Ensures that the NotarizationMethod specific invariants are hold
/// See fun `are_locked_notarization_invariants_ok()` and
/// `are_dynamic_notarization_invariants_ok()`
/// for more details.
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

/// Indicates if the invariants for `NotarizationMethod::Locked` are satisfied:
///
/// - `self.immutable_metadata.locking` must exist.
/// - `updated_lock` and `transfer_lock` must be `TimeLock::UntilDestroyed`.
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

/// Indicates if the invariants for `NotarizationMethod::Dynamic` are satisfied:
///
/// - Dynamic notarization can only have transfer locking or no
/// `immutable_metadata.locking`.
///   If `immutable_metadata.locking` exists, all locks except `transfer_lock`
///   must be `TimeLock::None`
///   and the `transfer_lock` must not be `TimeLock::None`.
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
