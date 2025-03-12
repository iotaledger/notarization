// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides dynamic notarization capabilities that can be freely updated by its owner
#[allow(lint(self_transfer))]
module iota_notarization::dynamic_notarization {
    use std::string::String;
    use iota::event;
    use iota::clock::Clock;
    use iota_notarization::notarization;

    // ===== Constants =====
    /// Cannot transfer a notarization that is not transferrable
    const ENotTransferrable: u64 = 3;
    /// Cannot transfer a locked notarization
    const ECannotTransferLocked: u64 = 4;

    /// Event emitted when a dynamic notarization is created
    public struct DynamicNotarizationCreated has copy, drop {
        /// ID of the `Notarization` object that was created
        notarization_obj_id: iota::object::ID,
    }

    /// Event emitted when a dynamic notarization is transferred
    public struct DynamicNotarizationTransferred has copy, drop {
        /// ID of the `Notarization` object that was transferred
        notarization_obj_id: iota::object::ID,
        /// Address of the new owner
        new_owner: address,
    }

    /// Create a new dynamic `Notarization`
    public fun new_dynamic_notarization<D: store + drop + copy>(
        state: notarization::State<D>,
        description: std::option::Option<String>,
        updateable_metadata: std::option::Option<String>,
        transferrable: bool,
        clock: &Clock,
        ctx: &mut iota::tx_context::TxContext
    ): notarization::Notarization<D> {
        notarization::new_dynamic_notarization(
            state,
            description,
            updateable_metadata,
            transferrable,
            clock,
            ctx
        )
    }

    /// Create and transfer a new dynamic `Notarization` to the sender
    public fun create_dynamic_notarization<D: store + drop + copy>(
        state: notarization::State<D>,
        description: std::option::Option<String>,
        updateable_metadata: std::option::Option<String>,
        transferrable: bool,
        clock: &Clock,
        ctx: &mut iota::tx_context::TxContext
    ) {
        // Use the core module to create and transfer the notarization
        let notarization = new_dynamic_notarization(state, description, updateable_metadata, transferrable, clock, ctx);

        let id = object::uid_to_inner(notarization.id());
        event::emit(DynamicNotarizationCreated { notarization_obj_id: id });

        notarization::transfer_notarization(notarization, tx_context::sender(ctx));
    }

    /// Transfer a dynamic notarization to a new owner
    /// Only works for dynamic notarizations that are marked as transferrable
    public fun transfer_dynamic_notarization<D: store + drop + copy>(
        self: notarization::Notarization<D>,
        recipient: address
    ) {
        // Ensure this is a dynamic notarization (not locked)
        assert!(!notarization::has_locking(&self), ECannotTransferLocked);

        // Ensure this notarization is transferrable
        assert!(notarization::is_transferrable(&self), ENotTransferrable);

        // Use the core module to transfer the notarization
        notarization::transfer_notarization(self, recipient);

        // Emit our own module-specific event
        let id = iota::object::id_from_address(recipient);
        event::emit(DynamicNotarizationTransferred {
            notarization_obj_id: id,
            new_owner: recipient
        });
    }

}
