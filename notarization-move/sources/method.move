// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Defines the `NotarizationMethod` enum used to distinguish between
/// Notarization Methods at runtime.
module iota_notarization::method;

use std::string::{Self, String};

/// Identifies the Notarization Method of a `Notarization`.
///
/// The set of Notarization Methods is closed in the current version of the
/// package but may be extended in future versions.
public enum NotarizationMethod has copy, drop, store {
    /// Method whose `state` and `updatable_metadata` can be updated after
    /// creation and which may optionally be transfer-locked.
    Dynamic,
    /// Method whose `state` and `updatable_metadata` are immutable after
    /// creation and whose destruction is gated by a `delete_lock`.
    Locked,
}

/// Returns the `Dynamic` Notarization Method.
public fun new_dynamic(): NotarizationMethod {
    NotarizationMethod::Dynamic
}

/// Returns the `Locked` Notarization Method.
public fun new_locked(): NotarizationMethod {
    NotarizationMethod::Locked
}

/// Returns `true` when `method` is `Dynamic`.
public fun is_dynamic(method: &NotarizationMethod): bool {
    match (method) {
        NotarizationMethod::Dynamic => true,
        NotarizationMethod::Locked => false,
    }
}

/// Returns `true` when `method` is `Locked`.
public fun is_locked(method: &NotarizationMethod): bool {
    match (method) {
        NotarizationMethod::Dynamic => false,
        NotarizationMethod::Locked => true,
    }
}

/// Returns the human-readable name of `method` as a `String`.
///
/// The result depends on the Notarization Method:
/// * `Dynamic`: `"DynamicNotarization"`.
/// * `Locked`: `"LockedNotarization"`.
public fun to_str(method: &NotarizationMethod): String {
    match (method) {
        NotarizationMethod::Dynamic => {
            string::utf8(b"DynamicNotarization")
        },
        NotarizationMethod::Locked => {
            string::utf8(b"LockedNotarization")
        },
    }
}
