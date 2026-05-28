// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Public entry surface for Dynamic-Notarizations: `Notarization<D>` objects
/// configured with the `Dynamic` Notarization Method, whose `state` and
/// `updatable_metadata` can be updated after creation and which may
/// optionally carry a transfer lock.
module iota_notarization::dynamic_notarization;

use iota::{clock::Clock, event};
use iota_notarization::notarization;
use std::string::String;
use tf_components::timelock::TimeLock;

// ===== Constants =====
/// Raised when `transfer` is called on a notarization whose `transfer_lock`
/// is currently active.
const ECannotTransferLocked: u64 = 0;

/// Emitted by `create` after a Dynamic-Notarization is created and
/// transferred to the sender.
public struct DynamicNotarizationCreated has copy, drop {
    /// Id of the newly created `Notarization` object.
    notarization_id: ID,
}

/// Emitted by `transfer` after a Dynamic-Notarization is transferred.
public struct DynamicNotarizationTransferred has copy, drop {
    /// Id of the transferred `Notarization` object.
    notarization_id: ID,
    /// Address of the new owner.
    recipient: address,
}

/// Creates a new Dynamic-Notarization `Notarization<D>` without transferring
/// it.
///
/// Delegates to `notarization::new_dynamic_notarization`; see that function
/// for the full contract.
///
/// Aborts with:
/// * any error documented by `notarization::new_dynamic_notarization`.
///
/// Returns the constructed `Notarization<D>`.
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

/// Creates a new Dynamic-Notarization `Notarization<D>` and transfers it to
/// the transaction sender.
///
/// Aborts with:
/// * any error documented by `notarization::new_dynamic_notarization`.
///
/// Emits a `DynamicNotarizationCreated` event on success.
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

/// Transfers `self` to `recipient`.
///
/// Permitted only when `is_transferable` returns `true` against `clock`,
/// i.e. when `self` has no `LockMetadata` or its `transfer_lock` is not
/// currently active.
///
/// Aborts with:
/// * `ECannotTransferLocked` when `is_transferable` is `false`.
///
/// Emits a `DynamicNotarizationTransferred` event on success.
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

/// Checks whether `self` may currently be transferred.
///
/// Returns `true` when `self` has no `LockMetadata` or when its
/// `transfer_lock` is not currently timelocked according to `clock`.
public fun is_transferable<D: store + drop + copy>(
    self: &notarization::Notarization<D>,
    clock: &Clock,
): bool {
    self.lock_metadata().is_none() || !self.is_transfer_locked(clock)
}
