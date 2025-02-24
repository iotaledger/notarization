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
    use iota_notarization::timelock_unlock_condition::{Self, TimelockUnlockCondition};
    use iota_notarization::lock_configuration::{Self, LockConfiguration};

    // ===== Constants =====
    /// Cannot update state while notarization is locked for updates
    const EUpdateWhileLocked: u64 = 0;
    /// Cannot destroy while notarization is locked for deletion
    const EDestroyWhileLocked: u64 = 1;

    // ===== Core Types =====
    /// A unified notarization type that can be either dynamic or locked
    public struct Notarization<S: store + drop> has key, store {
        id: UID,
        /// The state of the notarization that can be updated
        state: S,
        /// Variant-specific metadata
        immutable_metadata: ImmutableMetadata,
        /// Timestamp of the last state change
        last_state_change_at: u64,
        /// Counter for the number of state updates
        state_version_count: u64,
    }

    /// A Struct to handle different notarization variants
    public struct ImmutableMetadata has store {
        /// Timestamp when the notarization was created
        created_at: u64,
        /// Description of the notarization
        description: Option<String>,
        /// Optional lock metadata for locked notarization
        locking: Option<LockMetadata>,
    }

    // ===== Metadata Types =====
    /// Extended immutable metadata for locked notarizations
    public struct LockMetadata has store {
        /// Lock condition for state updates (unix_time = 0 means infinitely locked)
        update_lock: TimelockUnlockCondition,
        /// Lock condition for deletion (unix_time = 0 means infinitely locked)
        delete_lock: TimelockUnlockCondition,
    }

    // ===== State Types =====
    /// Default state type for notarizations to store arbitrary binary data
    public struct DefaultState has store, drop, copy {
        /// arbitrary binary data
        data: vector<u8>,
        /// Mutable metadata that can be updated together with the state data
        metadata: String,
    }

    // ===== Event Types =====
    /// Event emitted when the state of a notarization is updated
    public struct NotarizationUpdated has copy, drop {
        /// ID of the notarization object that was updated
        notarization_obj_id: ID,
        /// New version number after the update
        state_version_count: u64,
    }

    /// Event emitted when a notarization is created
    public struct NotarizationCreated has copy, drop {
        /// ID of the notarization object that was created
        notarization_obj_id: ID,
    }

    // ===== Constructor Functions =====
    /// Create a new DefaultState
    public fun new_default_state(data: vector<u8>, metadata: String): DefaultState {
        DefaultState { data, metadata }
    }


    /// Create lock metadata
    fun new_lock_metadata(
        clock: &Clock,
        lock_config: &LockConfiguration,
    ): LockMetadata {
        LockMetadata {
            update_lock: timelock_unlock_condition::new(
                lock_configuration::update_lock_period(lock_config),
                clock
            ),
            delete_lock: timelock_unlock_condition::new(
                lock_configuration::delete_lock_period(lock_config),
                clock
            ),
        }
    }

    // ===== Notarization Creation Functions =====
    /// Create a new dynamic notarization
    public fun new_dynamic_notarization<S: store + drop>(
        state: S,
        description: Option<String>,
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
            last_state_change_at: clock::timestamp_ms(clock),
            state_version_count: 0,
        }
    }

    /// Create a new locked notarization
    public fun new_locked_notarization<S: store + drop>(
        state: S,
        description: Option<String>,
        lock_config: LockConfiguration,
        clock: &Clock,
        ctx: &mut TxContext
    ): Notarization<S> {
        let lock_metadata = new_lock_metadata(clock,  &lock_config);

        Notarization<S> {
            id: object::new(ctx),
            state,
            immutable_metadata: ImmutableMetadata {
                created_at: clock::timestamp_ms(clock),
                description,
                locking: option::some(lock_metadata),
            },
            last_state_change_at: clock::timestamp_ms(clock),
            state_version_count: 0,
        }
    }

    /// Create and transfer a new dynamic notarization to the sender
    public fun create_dynamic_notarization<S: store + drop>(
        state: S,
        description: Option<String>,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let notarization = new_dynamic_notarization(state, description, clock, ctx);

        let id = object::uid_to_inner(&notarization.id);
        event::emit(NotarizationCreated { notarization_obj_id: id });

        transfer::transfer(notarization, tx_context::sender(ctx));
    }

    /// Create and transfer a new locked notarization to the sender
    public fun create_locked_notarization<S: store + drop>(
        state: S,
        description: Option<String>,
        lock_config: LockConfiguration,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let notarization = new_locked_notarization(state, description, lock_config, clock, ctx);

        let id = object::uid_to_inner(&notarization.id);
        event::emit(NotarizationCreated { notarization_obj_id: id });
        transfer::transfer(notarization, tx_context::sender(ctx));
    }

    // ===== State Management Functions =====
    /// Update the state of a notarization
    /// Will check locks if the notarization is a locked variant
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

    /// Destroy a notarization
    /// Will check locks if the notarization is a locked variant
    public fun destroy<S: drop + store>(
        self: Notarization<S>,
        clock: &Clock,
    ) {
        if (self.is_delete_locked(clock)){
            abort EDestroyWhileLocked
        };

        let Notarization { id, state: _, immutable_metadata: ImmutableMetadata {
            created_at: _, description: _, locking,
        }, last_state_change_at: _, state_version_count: _ } = self;

        if (locking.is_some()) {
            let LockMetadata { update_lock, delete_lock } = option::destroy_some(locking);

            timelock_unlock_condition::destroy_if_unlocked(update_lock, clock);
            timelock_unlock_condition::destroy_if_unlocked(delete_lock, clock);
        } else {
            // We know dynamic notarizations have no lock metadata
            option::destroy_none(locking);
        };

        object::delete(id);
    }

    // ===== Basic Getter Functions =====
    public fun state<S: store + drop>(self: &Notarization<S>): &S { &self.state }
    public fun is_locked<S: store + drop>(self: &Notarization<S>): bool { self.immutable_metadata.locking.is_some() }
    public fun created_at<S: store + drop>(self: &Notarization<S>): u64 { self.immutable_metadata.created_at }
    public fun last_change<S: store + drop>(self: &Notarization<S>): u64 { self.last_state_change_at }
    public fun version_count<S: store + drop>(self: &Notarization<S>): u64 { self.state_version_count }
    public fun description<S: store + drop>(self: &Notarization<S>): &Option<String> { &self.immutable_metadata.description }

    // ===== Lock-Related Getter Functions =====
    /// Get the lock metadata if this is a locked notarization
    public fun lock_metadata<S: store + drop>(self: &Notarization<S>): &Option<LockMetadata> {
        &self.immutable_metadata.locking
    }

    /// Check if the notarization is locked for updates (always false for dynamic variant)
    public fun is_update_locked<S: store + drop>(self: &Notarization<S>, clock: &Clock): bool {
        if (!self.immutable_metadata.locking.is_some()) {
            false
        } else {
            let lock_metadata = option::borrow(&self.immutable_metadata.locking);
            timelock_unlock_condition::is_timelocked(&lock_metadata.update_lock, clock)
        }
    }

    /// Check if the notarization is locked for deletion (always false for dynamic variant)
    public fun is_delete_locked<S: store + drop>(self: &Notarization<S>, clock: &Clock): bool {
        if (!self.immutable_metadata.locking.is_some()) {
            false
        } else {
            let lock_metadata = option::borrow(&self.immutable_metadata.locking);
            timelock_unlock_condition::is_timelocked(&lock_metadata.delete_lock, clock)
        }
    }

    /// Get the current lock configuration (none for dynamic variant)
    public fun lock_config<S: store + drop>(self: &Notarization<S>): Option<LockConfiguration> {
        if (!self.immutable_metadata.locking.is_some()) {
            option::none()
        } else {
            let lock_metadata = option::borrow(&self.immutable_metadata.locking);
            option::some(lock_configuration::new_lock_configuration(
                timelock_unlock_condition::unix_time(&lock_metadata.update_lock),
                timelock_unlock_condition::unix_time(&lock_metadata.delete_lock),
            ))
        }
    }
}