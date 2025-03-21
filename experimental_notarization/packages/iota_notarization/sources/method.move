// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides enum NotarizationType used to distinguish programmatically
/// between Notarization methods.
module iota_notarization::method;
use std::string::{Self, String};

// Indicates the Notarization method.
public enum NotarizationType has store, drop, copy {
    Dynamic,
    Locked
}

/// Returns a new NotarizationType::Dynamic.
public fun new_dynamic(): NotarizationType {
    NotarizationType::Dynamic
}

/// Returns a new NotarizationType::Locked.
public fun new_locked(): NotarizationType {
    NotarizationType::Locked
}

/// Returns true if the NotarizationType is Dynamic
public fun is_dynamic(method: &NotarizationType): bool {
    match (method) {
        NotarizationType::Dynamic => true,
        NotarizationType::Locked => false
    }
}

/// Returns true if the NotarizationType is Locked
public fun is_locked(method: &NotarizationType): bool {
    match (method) {
        NotarizationType::Dynamic => false,
        NotarizationType::Locked => true
    }
}

/// Returns the Notarization method as String
public fun to_str(method: &NotarizationType): String {
    match (method) {
        NotarizationType::Dynamic => {
            string::utf8(b"DynamicNotarization")
        },
        NotarizationType::Locked => {
            string::utf8(b"LockedNotarization")
        },
    }
}