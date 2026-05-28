// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Public entry surface for Locked-Notarizations: `Notarization<D>` objects
/// configured with the `Locked` Notarization Method, whose `state` and
/// `updatable_metadata` are immutable after creation and whose destruction
/// is gated by a `delete_lock`.
module iota_notarization::locked_notarization;

use iota::{clock::Clock, event};
use iota_notarization::notarization;
use std::string::String;
use tf_components::timelock::TimeLock;

/// Emitted by `create` after a Locked-Notarization is created and
/// transferred to the sender.
public struct LockedNotarizationCreated has copy, drop {
    /// Id of the newly created `Notarization` object.
    notarization_id: ID,
}

/// Creates a new Locked-Notarization `Notarization<D>` without transferring
/// it.
///
/// Delegates to `notarization::new_locked_notarization`; see that function
/// for the full contract.
///
/// Aborts with:
/// * any error documented by `notarization::new_locked_notarization`.
///
/// Returns the constructed `Notarization<D>`.
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

/// Creates a new Locked-Notarization `Notarization<D>` and transfers it to
/// the transaction sender.
///
/// Aborts with:
/// * any error documented by `notarization::new_locked_notarization`.
///
/// Emits a `LockedNotarizationCreated` event on success.
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
