// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides unified notarization capabilities with two variants:
/// 1. Dynamic - A basic notarization that can be freely updated by its owner
/// 2. Locked - A notarization with timelock controls for updates and deletion
#[allow(lint(self_transfer))]
module iota_notarization::notarization {
    use iota::event;
    use iota::clock::{Self, Clock};
    use std::string::String;
    use iota_notarization::timelock::{Self, TimeLock};

    // ===== Constants =====
    /// Cannot update state while notarization is locked for updates
    const EUpdateWhileLocked: u64 = 0;
    /// Cannot destroy while notarization is locked for deletion
    const EDestroyWhileLocked: u64 = 1;
    /// A delete_lock is not allowed to be set to be TimeLock::InfiniteLock
    const EInfiniteDeleteLockPeriod: u64 = 2;

    // ===== Core Type =====
    /// A unified notarization type that can be either dynamic or locked
    public struct Notarization<S: store + drop> has key, store {
        id: UID,
        /// The state of the `Notarization` that can be updated
        state: S,
        /// Immutable metadata
        immutable_metadata: ImmutableMetadata,
        /// Provides context or additional information for third parties
        updateable_metadata: Option<String>,
        /// Timestamp of the last state change
        last_state_change_at: u64,
        /// Counter for the number of state updates
        state_version_count: u64,
    }

    // ===== Metadata and Locking =====
    /// Gathers immutable fields defined when the `Notarization` object is created
    public struct ImmutableMetadata has store {
        /// Timestamp when the `Notarization` was created
        created_at: u64,
        /// Description of the `Notarization`
        description: Option<String>,
        /// Optional lock metadata for locked `Notarization`
        locking: Option<LockMetadata>,
    }

    /// Defines how a `Notarization` is locked.
    /// Can be used with the optional `ImmutableMetadata::locking`.
    public struct LockMetadata has store {
        /// Lock condition for state updates
        update_lock: TimeLock,
        /// Lock condition for deletion
        ///
        /// NOTE: delete lock cannot be infinite
        delete_lock: TimeLock,
    }

    // ===== Notarization State =====
    /// Can be used in the sense of a default type for the type argument S for Notarization<S>.
    /// Stores arbitrary binary data and some String metadata.
    /// Using DefaultState is just one option to use with Notarization<S>. You can use
    /// Notarization<S> with pure vector<u8>, String, ... or any custom type of your choice.
    public struct DefaultState has store, drop, copy {
        /// arbitrary binary data
        data: vector<u8>,
        /// Mutable metadata that can be updated together with the state data
        metadata: Option<String>,
    }

    // ===== Event Types =====
    /// Event emitted when the state of a `Notarization` is updated
    public struct NotarizationUpdated has copy, drop {
        /// ID of the `Notarization` object that was updated
        notarization_obj_id: ID,
        /// New version number after the update
        state_version_count: u64,
    }

    /// Event emitted when a `Notarization` is created
    public struct NotarizationCreated has copy, drop {
        /// ID of the `Notarization` object that was created
        notarization_obj_id: ID,
    }

    /// Event emitted when a `Notarization` is destroyed
    public struct NotarizationDestroyed has copy, drop {
        /// ID of the `Notarization` object that was destroyed
        notarization_obj_id: ID,
    }

    // ===== Constructor Functions =====
    /// Create a new DefaultState
    public fun new_default_state(data: vector<u8>, metadata: Option<String>): DefaultState {
        DefaultState { data, metadata }
    }


    /// Create lock metadata
    fun new_lock_metadata(
        delete_lock: TimeLock,
        update_lock: TimeLock,
    ): LockMetadata {
        LockMetadata {
            update_lock: update_lock,
            delete_lock: delete_lock
        }
    }

    // ===== Notarization Creation Functions =====
    /// Create a new dynamic `Notarization`
    public fun new_dynamic_notarization<S: store + drop>(
        state: S,
        description: Option<String>,
        updateable_metadata: Option<String>,
        clock: &Clock,
        ctx: &mut TxContext
    ): Notarization<S> {
        Notarization<S> {
            id: object::new(ctx),
            state,
            immutable_metadata: ImmutableMetadata {
                created_at: clock::timestamp_ms(clock),
                description,
                locking: option::none(),
            },
            updateable_metadata,
            last_state_change_at: clock::timestamp_ms(clock),
            state_version_count: 0,
        }
    }

    /// Create a new locked `Notarization`
    public fun new_locked_notarization<S: store + drop>(
        state: S,
        description: Option<String>,
        updateable_metadata: Option<String>,
        delete_lock: TimeLock,
        update_lock: TimeLock,
        clock: &Clock,
        ctx: &mut TxContext
    ): Notarization<S> {
        // Assert that the delete lock is not infinite
        assert!(!delete_lock.is_infinite_lock(), EInfiniteDeleteLockPeriod);

        Notarization<S> {
            id: object::new(ctx),
            state,
            immutable_metadata: ImmutableMetadata {
                created_at: clock::timestamp_ms(clock),
                description,
                locking: option::some(new_lock_metadata(delete_lock, update_lock)),
            },
            updateable_metadata,
            last_state_change_at: clock::timestamp_ms(clock),
            state_version_count: 0,
        }
    }

    /// Create and transfer a new dynamic `Notarization` to the sender
    public fun create_dynamic_notarization<S: store + drop>(
        state: S,
        description: Option<String>,
        updateable_metadata: Option<String>,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let notarization = new_dynamic_notarization(state, description, updateable_metadata, clock, ctx);

        let id = object::uid_to_inner(&notarization.id);
        event::emit(NotarizationCreated { notarization_obj_id: id });

        transfer::transfer(notarization, tx_context::sender(ctx));
    }

    /// Create and transfer a new locked notarization to the sender
    public fun create_locked_notarization<S: store + drop>(
        state: S,
        description: Option<String>,
        updateable_metadata: Option<String>,
        delete_lock: TimeLock,
        update_lock: TimeLock,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let notarization = new_locked_notarization(state, description, updateable_metadata, delete_lock, update_lock, clock, ctx);

        let id = object::uid_to_inner(&notarization.id);
        event::emit(NotarizationCreated { notarization_obj_id: id });
        transfer::transfer(notarization, tx_context::sender(ctx));
    }

    // ===== State Management Functions =====
    /// Update the state of a `Notarization`
    /// Will check locks if the `Notarization` is a locked variant
    public fun update_state<S: store + drop>(
        self: &mut Notarization<S>,
        new_state: S,
        clock: &Clock,
    ) {
        if (self.is_update_locked(clock)) {
            abort EUpdateWhileLocked
        };

        self.state = new_state;
        self.last_state_change_at = clock::timestamp_ms(clock);
        self.state_version_count = self.state_version_count + 1;

        event::emit(NotarizationUpdated {
            notarization_obj_id: object::uid_to_inner(&self.id),
            state_version_count: self.state_version_count,
        });
    }

    /// Destroy a `Notarization`
    /// Will check locks if the `Notarization` is a locked variant
    public fun destroy<S: drop + store>(
        self: Notarization<S>,
        clock: &Clock,
    ) {
        assert!(!self.is_delete_locked(clock), EDestroyWhileLocked);

        let Notarization { id, state: _, immutable_metadata: ImmutableMetadata {
            created_at: _, description: _, locking,
        }, updateable_metadata: _, last_state_change_at: _, state_version_count: _ } = self;

        if (locking.is_some()) {
            let LockMetadata { update_lock, delete_lock } = option::destroy_some(locking);

            // destroy the locks
            timelock::destroy_if_unlocked_or_infinite_lock(update_lock, clock);
            timelock::destroy_if_unlocked_or_infinite_lock(delete_lock, clock);
        } else {
            // We know dynamic Notarizations have no lock metadata
            option::destroy_none(locking);
        };

        let id_inner = object::uid_to_inner(&id);
        object::delete(id);
        event::emit(NotarizationDestroyed { notarization_obj_id: id_inner });
    }

    // ===== Metadata Management Functions =====
    /// Update the updateable metadata of a `Notarization`
    /// This does not affect the state version count
    /// Will check locks if the `Notarization` is a locked variant (uses the same lock as state updates)
    public fun update_metadata<S: store + drop>(
        self: &mut Notarization<S>,
        new_metadata: Option<String>,
        clock: &Clock,
    ) {
        if (self.is_update_locked(clock)) {
            abort EUpdateWhileLocked
        };

        self.updateable_metadata = new_metadata;
    }

    /// Get the updateable metadata of a `Notarization`
    public fun get_updateable_metadata<S: store + drop>(self: &Notarization<S>): &Option<String> {
        &self.updateable_metadata
    }

    // ===== Basic Getter Functions =====
    public fun state<S: store + drop>(self: &Notarization<S>): &S { &self.state }
    public fun is_locked<S: store + drop>(self: &Notarization<S>): bool { self.immutable_metadata.locking.is_some() }
    public fun created_at<S: store + drop>(self: &Notarization<S>): u64 { self.immutable_metadata.created_at }
    public fun last_change<S: store + drop>(self: &Notarization<S>): u64 { self.last_state_change_at }
    public fun version_count<S: store + drop>(self: &Notarization<S>): u64 { self.state_version_count }
    public fun description<S: store + drop>(self: &Notarization<S>): &Option<String> { &self.immutable_metadata.description }

    // ===== Lock-Related Getter Functions =====
    /// Get the lock metadata if this is a locked Notarization
    public fun lock_metadata<S: store + drop>(self: &Notarization<S>): &Option<LockMetadata> {
        &self.immutable_metadata.locking
    }

    /// Check if the `Notarization` is locked for updates (always false for dynamic variant)
    public fun is_update_locked<S: store + drop>(self: &Notarization<S>, clock: &Clock): bool {
        if (!self.immutable_metadata.locking.is_some()) {
            false
        } else {
            let lock_metadata = option::borrow(&self.immutable_metadata.locking);
            timelock::is_timelocked(&lock_metadata.update_lock, clock)
        }
    }

    /// Check if the `Notarization` is locked for deletion (always false for dynamic variant)
    public fun is_delete_locked<S: store + drop>(self: &Notarization<S>, clock: &Clock): bool {
        if (!self.immutable_metadata.locking.is_some()) {
            false
        } else {
            let lock_metadata = option::borrow(&self.immutable_metadata.locking);
            timelock::is_timelocked(&lock_metadata.delete_lock, clock)
        }
    }
}