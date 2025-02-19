module iota_notarization::notarization {
    use iota::event;
    use iota::clock::{Self, Clock};

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
    public struct ImmutableMetadata has store, drop, copy {
        /// Timestamp when the notarization was created
        created_at: u64,
        /// Timestamp of the last state change
        last_state_change_at: u64,
        /// Counter for the number of state updates
        state_version_count: u64,
    }

    /// Represents a notarization record that can be dynamically updated
    /// The generic type T represents the state data type
    public struct Notarization has key {
        id: UID,
        /// The state of the notarization that can be updated
        state: vector<u8>,
        /// Immutable metadata containing timestamps and version info
        metadata: ImmutableMetadata,
    }

    /// Create new immutable metadata
    fun new_metadata(clock: &Clock): ImmutableMetadata {
        let timestamp = clock::timestamp_ms(clock);
        ImmutableMetadata {
            created_at: timestamp,
            last_state_change_at: timestamp,
            state_version_count: 0,
        }
    }

    /// Create a new notarization record
    public fun new(
        state: vector<u8>,
        clock: &Clock,
        ctx: &mut TxContext
    ): Notarization {
        Notarization {
            id: object::new(ctx),
            state,
            metadata: new_metadata(clock),
        }
    }

    /// Create and transfer a new notarization record to the sender
    public entry fun create_and_transfer(
        state: vector<u8>,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let notarization = new(state, clock, ctx);
        let id = object::uid_to_inner(&notarization.id);

        // Emit creation event
        event::emit(NotarizationCreated {
            notarization_obj_id: id,
        });

        transfer::transfer(notarization, tx_context::sender(ctx));
    }

    /// Update the state of a notarization
    /// Only the owner can update the state
    public entry fun update_state(
        self: &mut Notarization,
        new_state: vector<u8>,
        clock: &Clock,
    ) {
        // Update the state
        self.state = new_state;

        // Update metadata
        self.metadata.last_state_change_at = clock::timestamp_ms(clock);
        self.metadata.state_version_count = self.metadata.state_version_count + 1;

        // Emit update event
        event::emit(NotarizationUpdated {
            notarization_obj_id: object::uid_to_inner(&self.id),
            state_version_count: self.metadata.state_version_count,
        });
    }

    /// Destroy an empty notarization record
    public entry fun destroy_empty(
        self: Notarization,
    ) {

        let Notarization {
            id,
            state: _,
            metadata: _,
        } = self;

        // Clean up the object
        object::delete(id);
    }

    // === Getter Functions ===

    /// Get the state of the notarization
    public fun state(self: &Notarization): &vector<u8> {
        &self.state
    }

    /// Get the immutable metadata
    public fun metadata(self: &Notarization): &ImmutableMetadata {
        &self.metadata
    }

    /// Get the creation timestamp
    public fun created_at(self: &Notarization): u64 {
        self.metadata.created_at
    }

    /// Get the last state change timestamp
    public fun last_state_change_at(self: &Notarization): u64 {
        self.metadata.last_state_change_at
    }

    /// Get the state version count
    public fun state_version_count(self: &Notarization): u64 {
        self.metadata.state_version_count
    }

}