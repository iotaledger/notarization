module iota_notarization::notarization {
    use iota::event;
    use iota::clock::{Self, Clock};
    use std::string::String;

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
        /// Description of the notarization
        description: String,
    }

    /// Can be used for Notarization<S> to store arbitrary binary data
    public struct DefaultState has store, drop, copy {
        /// arbitrary binary data
        data: vector<u8>,
        /// Mutable metadata that can be updated together with the state data
        metadata: String,
    }

    /// Represents a notarization record that can be dynamically updated
    /// The generic type T represents the state data type
    public struct Notarization<S> has key {
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
    fun new_metadata(clock: &Clock, description: String): ImmutableMetadata {
        let timestamp = clock::timestamp_ms(clock);
        ImmutableMetadata {
            created_at: timestamp,
            description,
        }
    }

    /// Create a new notarization record
    public fun new<S>(
        state: S,
        description: String,
        clock: &Clock,
        ctx: &mut TxContext
    ): Notarization<S> {
        Notarization<S> {
            id: object::new(ctx),
            state,
            immutable_metadata: new_metadata(clock, description),
            last_state_change_at: clock::timestamp_ms(clock),
            state_version_count: 0, // Initial state
        }
    }

    /// Create and transfer a new notarization record to the sender
    public entry fun create_and_transfer<S: store + drop + copy>(
        state: S,
        description: String,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        let notarization = new(state, description, clock, ctx);
        let id = object::uid_to_inner(&notarization.id);

        // Emit creation event
        event::emit(NotarizationCreated {
            notarization_obj_id: id,
        });

        transfer::transfer(notarization, tx_context::sender(ctx));
    }

    /// Update the state of a notarization
    /// Only the owner can update the state
    public entry fun update_state<S: store + drop + copy>(
        self: &mut Notarization<S>,
        new_state: S,
        clock: &Clock,
    ) {
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

    /// Destroy an empty notarization record
    public entry fun destroy_empty<S>(
        self: Notarization<S>,
    ) {

        let Notarization<S> {
            id,
            state: _,
            immutable_metadata: _,
            last_state_change_at: _,
            state_version_count: _,
        } = self;

        // Clean up the object
        object::delete(id);
    }

    // === Getter Functions ===

    /// Get the state of the notarization
    public fun state<S>(self: &Notarization<S>): &S {
        &self.state
    }

    /// Get the immutable metadata
    public fun immutable_metadata<S>(self: &Notarization<S>): &ImmutableMetadata {
        &self.immutable_metadata
    }

    /// Get the creation timestamp
    public fun created_at<S>(self: &Notarization<S>): u64 {
        self.immutable_metadata.created_at
    }

    /// Get the last state change timestamp
    public fun last_state_change_at<S>(self: &Notarization<S>): u64 {
        self.last_state_change_at
    }

    /// Get the state version count
    public fun state_version_count<S>(self: &Notarization<S>): u64 {
        self.state_version_count
    }

    /// Get the description of the notarization
    public fun description<S>(self: &Notarization<S>): &String {
        &self.immutable_metadata.description
    }

}