// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides core notarization capabilities to be used by
/// locked_notarization and dynamic_notarization modules
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
    public struct Notarization<D: store + drop + copy> has key {
        id: UID,
        /// The state of the `Notarization` that can be updated
        state: State<D>,
        /// Variant-specific metadata
        immutable_metadata: ImmutableMetadata,
        /// Provides context or additional information for third parties
        updateable_metadata: Option<String>,
        /// Timestamp of the last state change
        last_state_change_at: u64,
        /// Counter for the number of state updates
        state_version_count: u64,
        /// Whether this notarization can be transferred to another owner
        /// Only applicable for dynamic notarizations
        transferrable: bool,
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
    /// Represents the state of a Notarization that can be updated
    /// Contains arbitrary data and metadata that can be updated by the owner
    /// The arbitrary data is stored in a generic type D.
    public struct State<D: store + drop + copy> has store, drop, copy {
        /// The data being notarized
        data: D,
        /// Mutable metadata that can be updated together with the state data
        metadata: Option<String>,
    }

    // ===== Event Types =====
    /// Event emitted when the state of a `Notarization` is updated
    public struct NotarizationUpdated<D: store + drop + copy> has copy, drop {
        /// ID of the `Notarization` object that was updated
        notarization_obj_id: ID,
        /// New version number after the update
        state_version_count: u64,
        /// Updated State
        updated_state: State<D>
    }

    /// Event emitted when a `Notarization` is destroyed
    public struct NotarizationDestroyed has copy, drop {
        /// ID of the `Notarization` object that was destroyed
        notarization_obj_id: ID,
    }

    // ===== Constructor Functions =====
    /// Create a new state from a vector<u8> data
    public fun new_state_from_vector(data: vector<u8>, metadata: Option<String>): State<vector<u8>> {
        State { data, metadata }
    }

    /// Create state from a string data
    public fun new_state_from_string(data: String, metadata: Option<String>): State<String> {
        State { data, metadata }
    }

    /// Create lock metadata
    public fun new_lock_metadata(
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
    public(package) fun new_dynamic_notarization<D: store + drop + copy>(
        state: State<D>,
        description: Option<String>,
        updateable_metadata: Option<String>,
        transferrable: bool,
        clock: &Clock,
        ctx: &mut TxContext
    ): Notarization<D> {
        Notarization<D> {
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
            transferrable,
        }
    }

    /// Create a new locked `Notarization`
    public(package) fun new_locked_notarization<D: store + drop + copy>(
        state: State<D>,
        description: Option<String>,
        updateable_metadata: Option<String>,
        delete_lock: TimeLock,
        update_lock: TimeLock,
        clock: &Clock,
        ctx: &mut TxContext
    ): Notarization<D> {
        // Assert that the delete lock is not infinite
        assert!(!delete_lock.is_infinite_lock(), EInfiniteDeleteLockPeriod);

        Notarization<D> {
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
            transferrable: false, // Locked notarizations are never transferrable
        }
    }

    // ===== State Management Functions =====
    /// Update the state of a `Notarization`
    /// Will check locks if the `Notarization` is a locked variant
    public fun update_state<D: store + drop + copy>(
        self: &mut Notarization<D>,
        new_state: State<D>,
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
            updated_state: new_state
        });
    }

    /// Destroy a `Notarization`
    /// Will check locks if the `Notarization` is a locked variant
    public fun destroy<D: drop + store + copy>(
        self: Notarization<D>,
        clock: &Clock,
    ) {
        assert!(!self.is_delete_locked(clock), EDestroyWhileLocked);

        let Notarization { id, state: _, immutable_metadata: ImmutableMetadata {
            created_at: _, description: _, locking,
        }, updateable_metadata: _, last_state_change_at: _, state_version_count: _, transferrable: _ } = self;

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

    /// Re-exports the transfer function from the core module
    ///
    /// Workaround for transferability
    public(package) fun transfer_notarization<D: store + drop + copy>(
        self: Notarization<D>,
        recipient: address
    ) {
        transfer::transfer(self, recipient);
    }

    // ===== Metadata Management Functions =====
    /// Update the updateable metadata of a `Notarization`
    /// This does not affect the state version count
    /// Will check locks if the `Notarization` is a locked variant (uses the same lock as state updates)
    public fun update_metadata<D: store + drop + copy>(
        self: &mut Notarization<D>,
        new_metadata: Option<String>,
        clock: &Clock,
    ) {
        if (self.is_update_locked(clock)) {
            abort EUpdateWhileLocked
        };

        self.updateable_metadata = new_metadata;
    }

    /// Get the updateable metadata of a `Notarization`
    public fun get_updateable_metadata<D: store + drop + copy>(self: &Notarization<D>): &Option<String> {
        &self.updateable_metadata
    }

    // ===== Basic Getter Functions =====
    public fun id<D: store + drop + copy>(self: &Notarization<D>): &UID { &self.id }
    public fun state<D: store + drop + copy>(self: &Notarization<D>): &State<D> { &self.state }
    public fun has_locking<D: store + drop + copy>(self: &Notarization<D>): bool { self.immutable_metadata.locking.is_some() }
    public fun created_at<D: store + drop + copy>(self: &Notarization<D>): u64 { self.immutable_metadata.created_at }
    public fun last_change<D: store + drop + copy>(self: &Notarization<D>): u64 { self.last_state_change_at }
    public fun version_count<D: store + drop + copy>(self: &Notarization<D>): u64 { self.state_version_count }
    public fun description<D: store + drop + copy>(self: &Notarization<D>): &Option<String> { &self.immutable_metadata.description }
    public fun is_transferrable<D: store + drop + copy>(self: &Notarization<D>): bool { !self.has_locking() && self.transferrable }

    // ===== Lock-Related Getter Functions =====
    /// Get the lock metadata if this is a locked Notarization
    public fun lock_metadata<D: store + drop + copy>(self: &Notarization<D>): &Option<LockMetadata> {
        &self.immutable_metadata.locking
    }

    /// Check if the `Notarization` is locked for updates (always false for dynamic variant)
    public fun is_update_locked<D: store + drop + copy>(self: &Notarization<D>, clock: &Clock): bool {
        if (!self.immutable_metadata.locking.is_some()) {
            false
        } else {
            let lock_metadata = option::borrow(&self.immutable_metadata.locking);
            timelock::is_timelocked(&lock_metadata.update_lock, clock)
        }
    }

    /// Check if the `Notarization` is locked for deletion (always false for dynamic variant)
    public fun is_delete_locked<D: store + drop + copy>(self: &Notarization<D>, clock: &Clock): bool {
        if (!self.immutable_metadata.locking.is_some()) {
            false
        } else {
            let lock_metadata = option::borrow(&self.immutable_metadata.locking);
            timelock::is_timelocked(&lock_metadata.delete_lock, clock)
        }
    }
}