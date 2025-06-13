// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides dynamic notarization capabilities that can be freely updated by its owner
module iota_notarization::dynamic_notarization;

use iota::{clock::Clock, event};
use iota_notarization::{notarization, timelock::TimeLock};
use std::string::String;

// ===== Constants =====
/// Cannot transfer a locked notarization
const ECannotTransferLocked: u64 = 0;

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
    recipient: address,
}

/// Create a new dynamic `Notarization`
public fun new<D: store + drop + copy>(
    state: notarization::State<D>,
    immutable_description: Option<String>,
    updatable_metadata: Option<String>,
    transfer_lock: TimeLock,
    clock: &Clock,
    ctx: &mut TxContext,
): notarization::Notarization<D> {
    notarization::new_dynamic_notarization(
        state,
        immutable_description,
        updatable_metadata,
        transfer_lock,
        clock,
        ctx,
    )
}

/// Create and transfer a new dynamic `Notarization` to the sender
public fun create<D: store + drop + copy>(
    state: notarization::State<D>,
    immutable_description: Option<String>,
    updatable_metadata: Option<String>,
    transfer_lock: TimeLock,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    // Use the core module to create and transfer the notarization
    let notarization = new(
        state,
        immutable_description,
        updatable_metadata,
        transfer_lock,
        clock,
        ctx,
    );

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
    _: &mut TxContext,
) {
    // Ensure this notarization is transferrable
    assert!(is_transferable(&self, clock), ECannotTransferLocked);

    notarization::transfer_notarization(self, recipient);

    let id = object::id_from_address(recipient);
    event::emit(DynamicNotarizationTransferred {
        notarization_id: id,
        recipient,
    });
}

/// Check if the notarization is transferable
public fun is_transferable<D: store + drop + copy>(
    self: &notarization::Notarization<D>,
    clock: &Clock,
): bool {
    self.lock_metadata().is_none() || !self.is_transfer_locked(clock)
}
