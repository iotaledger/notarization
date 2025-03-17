// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides locked notarization capabilities with timelock controls for updates and deletion
#[allow(lint(self_transfer))]
module iota_notarization::locked_notarization {
    use std::string::String;
    use iota::event;
    use iota::clock::Clock;
    use iota_notarization::timelock::TimeLock;
    use iota_notarization::notarization;

    /// Event emitted when a locked notarization is created
    public struct LockedNotarizationCreated has copy, drop {
        /// ID of the `Notarization` object that was created
        notarization_obj_id: ID,
    }

    /// Create a new locked `Notarization`
    public fun new_locked_notarization<D: store + drop + copy>(
        state: notarization::State<D>,
        description: Option<String>,
        updateable_metadata:Option<String>,
        delete_lock: Option<TimeLock>,
        clock: &Clock,
        ctx: &mut iota::tx_context::TxContext
    ): notarization::Notarization<D> {
        notarization::new_locked_notarization(
            state,
            description,
            updateable_metadata,
            delete_lock,
            clock,
            ctx
        )
    }

    /// Create and transfer a new locked notarization to the sender
    public fun create_locked_notarization<D: store + drop + copy>(
        state: notarization::State<D>,
        description: Option<String>,
        updateable_metadata: Option<String>,
        delete_lock: Option<TimeLock>,
        clock: &Clock,
        ctx: &mut iota::tx_context::TxContext
    ) {
        let notarization = new_locked_notarization(state, description, updateable_metadata, delete_lock,  clock, ctx);

        let id = object::uid_to_inner(notarization.id());
        event::emit(LockedNotarizationCreated { notarization_obj_id: id });
        notarization::transfer_notarization(notarization, tx_context::sender(ctx));
    }
}
