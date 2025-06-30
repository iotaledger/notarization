// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides locked notarization capabilities with timelock controls for updates and deletion
module iota_notarization::locked_notarization;

use iota::{clock::Clock, event};
use iota_notarization::{notarization, timelock::TimeLock};
use std::string::String;

/// Event emitted when a locked notarization is created
public struct LockedNotarizationCreated has copy, drop {
    /// ID of the `Notarization` object that was created
    notarization_id: ID,
}

/// Create a new locked `Notarization`
public fun new<D: store + drop + copy>(
    state: notarization::State<D>,
    immutable_description: Option<String>,
    updatable_metadata: Option<String>,
    delete_lock: TimeLock,
    clock: &Clock,
    ctx: &mut TxContext,
): notarization::Notarization<D> {
    notarization::new_locked_notarization(
        state,
        immutable_description,
        updatable_metadata,
        delete_lock,
        clock,
        ctx,
    )
}

/// Create and transfer a new locked notarization to the sender
public fun create<D: store + drop + copy>(
    state: notarization::State<D>,
    immutable_description: Option<String>,
    updatable_metadata: Option<String>,
    delete_lock: TimeLock,
    clock: &Clock,
    ctx: &mut TxContext,
) {
    let notarization = new(
        state,
        immutable_description,
        updatable_metadata,
        delete_lock,
        clock,
        ctx,
    );

    let id = object::uid_to_inner(notarization.id());

    event::emit(LockedNotarizationCreated { notarization_id: id });

    notarization::transfer_notarization(notarization, tx_context::sender(ctx));
}
