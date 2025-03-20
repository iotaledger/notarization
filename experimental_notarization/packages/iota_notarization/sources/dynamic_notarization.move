// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides dynamic notarization capabilities that can be freely updated by its owner
#[allow(lint(self_transfer))]
module iota_notarization::dynamic_notarization {
    use std::string::String;
    use iota::event;
    use iota::clock::Clock;
    use iota_notarization::notarization;
    use iota_notarization::timelock::TimeLock;

    // ===== Constants =====
    /// Cannot transfer a notarization that is not transferrable
    const ENotTransferrable: u64 = 0;
    /// Cannot transfer a locked notarization
    const ECannotTransferLocked: u64 = 1;

    /// Event emitted when a dynamic notarization is created
    public struct DynamicNotarizationCreated has copy, drop {
        /// ID of the `Notarization` object that was created
        notarization_id: ID,
    }

    /// Event emitted when a dynamic notarization is transferred
    public struct DynamicNotarizationTransferred has copy, drop {
        /// ID of the `Notarization` object that was transferred
        notarization_id: ID,
        /// Address of the new owner
        new_owner: address,
    }

    /// Create a new dynamic `Notarization`
    public fun new<D: store + drop + copy>(
        state: notarization::State<D>,
        immutable_description: Option<String>,
        updateable_metadata: Option<String>,
        transfer_lock: Option<TimeLock>,
        clock: &Clock,
        ctx: &mut TxContext
    ): notarization::Notarization<D> {
        notarization::new_dynamic_notarization(
            state,
            immutable_description,
            updateable_metadata,
            transfer_lock,
            clock,
            ctx
        )
    }

    /// Create and transfer a new dynamic `Notarization` to the sender
    public fun create<D: store + drop + copy>(
        state: notarization::State<D>,
        immutable_description: Option<String>,
        updateable_metadata: Option<String>,
        transfer_lock: Option<TimeLock>,
        clock: &Clock,
        ctx: &mut TxContext
    ) {
        // Use the core module to create and transfer the notarization
        let notarization = new(state, immutable_description, updateable_metadata, transfer_lock, clock, ctx);

        let id = object::uid_to_inner(notarization.id());
        event::emit(DynamicNotarizationCreated { notarization_id: id });

        notarization::transfer_notarization(notarization, tx_context::sender(ctx));
    }

    /// Transfer a dynamic notarization to a new owner
    /// Only works for dynamic notarizations that are marked as transferrable
    public fun transfer<D: store + drop + copy>(
        self: notarization::Notarization<D>,
        recipient: address,
        clock: &Clock,
        _: &mut TxContext
    ) {
        // Ensure this is a dynamic notarization (not locked)
        assert!(self.lock_metadata().is_none(), ECannotTransferLocked);

        // Ensure this notarization is transferrable
        assert!(is_transferable(&self, clock), ENotTransferrable);

        // Ensure the notarized object is not transfer locked
        assert!(self.is_transfer_locked(clock), ECannotTransferLocked);

        // Use the core module to transfer the notarization
        notarization::transfer_notarization(self, recipient);

        // Emit our own module-specific event
        let id = object::id_from_address(recipient);
        event::emit(DynamicNotarizationTransferred {
            notarization_id: id,
            new_owner: recipient
        });
    }

    /// Check if the notarization is transferable
    public fun is_transferable<D: store + drop + copy>(self: &notarization::Notarization<D>, clock: &Clock): bool {
        self.lock_metadata().is_none() || !self.is_transfer_locked(clock)
    }

}
