module iota_notarization::locked_notarization {
    use iota::event;
    use iota::clock::{Self, Clock};
    use std::string::String;
    use stardust::timelock_unlock_condition::{Self, TimelockUnlockCondition};


    // ===== Errors =====
    /// Cannot update state while notarization is locked
    const EUpdateWhileLocked: u64 = 0;
    /// Cannot destroy while notarization is locked
    const EDestroyWhileLocked: u64 = 1;

    // ===== Events =====
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

    // ===== Notarization =====
    /// Represents the immutable metadata of a notarization
    public struct ImmutableMetadata has store {
        /// Timestamp when the notarization was created
        created_at: u64,
        /// Description of the notarization
        description: Option<String>,
        /// Lock condition
        lock_condition: TimelockUnlockCondition,
    }

    /// Can be used for LockedNotarization<S> to store arbitrary binary data
    public struct DefaultState has store, drop, copy {
        /// arbitrary binary data
        data: vector<u8>,
        /// Mutable metadata that can be updated together with the state data
        metadata: String,
    }

    public fun new_default_state(data: vector<u8>, metadata: String): DefaultState {
        DefaultState {
            data,
            metadata,
        }
    }

    /// Represents a notarization record that can be dynamically updated
    /// The generic type T represents the state data type
    public struct LockedNotarization<S> has key {
        id: UID,
        /// The state of the notarization that can be updated
        state: S,
        /// Immutable metadata containing timestamps and version info
        immutable_metadata: ImmutableMetadata,
        /// Timestamp of the last state change
        last_state_change_at: u64,
        /// Counter for the number of state updates
        state_version_count: u64,
    }

    /// Create new immutable metadata
    fun new_metadata(clock: &Clock, description: Option<String>, lock: TimelockUnlockCondition): ImmutableMetadata {
        let timestamp = clock::timestamp_ms(clock);
        ImmutableMetadata {
            created_at: timestamp,
            description,
            lock_condition: lock,
        }
    }

    /// Create a new notarization record
    public fun new<S>(
        state: S,
        description: Option<String>,
        lock: TimelockUnlockCondition,
        clock: &Clock,
        ctx: &mut TxContext
    ): LockedNotarization<S> {
        LockedNotarization<S> {
            id: object::new(ctx),
            state,
            immutable_metadata: new_metadata(clock, description, lock),
            last_state_change_at: clock::timestamp_ms(clock),
            state_version_count: 0, // Initial state
        }
    }

    /// Create and transfer a new notarization record to the sender
    public fun create_and_transfer<S: store + drop + copy>(
        state: S,
        description: Option<String>,
        lock: TimelockUnlockCondition,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let notarization = new(state, description, lock, clock, ctx);
        let id = object::uid_to_inner(&notarization.id);

        // Emit creation event
        event::emit(NotarizationCreated {
            notarization_obj_id: id,
        });

        transfer::transfer(notarization, tx_context::sender(ctx));
    }

    /// Update the state of a notarization
    /// Only the owner can update the state and only if the timelock has expired
    public fun update_state<S: store + drop + copy>(
        self: &mut LockedNotarization<S>,
        new_state: S,
        clock: &Clock,
        ctx: &TxContext,
    ) {
        // Check if the notarization is still locked
        assert!(
            !timelock_unlock_condition::is_timelocked(&self.immutable_metadata.lock_condition, ctx),
            EUpdateWhileLocked
        );

        // Update the state
        self.state = new_state;

        // Update metadata
        self.last_state_change_at = clock::timestamp_ms(clock);
        self.state_version_count = self.state_version_count + 1;

        // Emit update event
        event::emit(NotarizationUpdated {
            notarization_obj_id: object::uid_to_inner(&self.id),
            state_version_count: self.state_version_count,
        });
    }

    /// Destroy a notarization record
    /// Can only be done after the timelock has expired
    public fun destroy_empty<S: drop + store + copy>(
        self: LockedNotarization<S>,
        ctx: &TxContext,
    ) {
        // Check if the notarization is still locked
        assert!(
            !timelock_unlock_condition::is_timelocked(&self.immutable_metadata.lock_condition, ctx),
            EDestroyWhileLocked
        );

        let LockedNotarization<S> {
            id,
            state: _,
            immutable_metadata: ImmutableMetadata {
                created_at: _,
                description: _,
                lock_condition,
            },
            last_state_change_at: _,
            state_version_count: _,
        } = self;

        // Drop the lock condition explicitly since we know it has store ability and we know the notarization is unlocked
        timelock_unlock_condition::unlock(lock_condition, ctx);

        // Clean up the object
        object::delete(id);
    }

    // === Getter Functions ===

    /// Get the state of the notarization
    public fun state<S>(self: &LockedNotarization<S>): &S {
        &self.state
    }

    /// Get the immutable metadata
    public fun immutable_metadata<S>(self: &LockedNotarization<S>): &ImmutableMetadata {
        &self.immutable_metadata
    }

    /// Get the creation timestamp
    public fun created_at<S>(self: &LockedNotarization<S>): u64 {
        self.immutable_metadata.created_at
    }

    /// Get the last state change timestamp
    public fun last_state_change_at<S>(self: &LockedNotarization<S>): u64 {
        self.last_state_change_at
    }

    /// Get the state version count
    public fun state_version_count<S>(self: &LockedNotarization<S>): u64 {
        self.state_version_count
    }

    /// Get the description of the notarization
    public fun description<S>(self: &LockedNotarization<S>): &Option<String> {
        &self.immutable_metadata.description
    }

    /// Get the lock condition
    public fun lock_condition<S>(self: &LockedNotarization<S>): &TimelockUnlockCondition {
        &self.immutable_metadata.lock_condition
    }

    /// Check if the notarization is locked
    public fun is_locked<S>(self: &LockedNotarization<S>, ctx: &TxContext): bool {
        timelock_unlock_condition::is_timelocked(&self.immutable_metadata.lock_condition, ctx)
    }
}