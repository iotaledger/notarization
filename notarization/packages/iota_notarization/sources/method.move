// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides enum NotarizationMethod used to distinguish programmatically
/// between Notarization methods.
module iota_notarization::method;

use std::string::{Self, String};

// Indicates the Notarization method.
public enum NotarizationMethod has copy, drop, store {
    Dynamic,
    Locked,
}

/// Returns a new NotarizationMethod::Dynamic.
public fun new_dynamic(): NotarizationMethod {
    NotarizationMethod::Dynamic
}

/// Returns a new NotarizationMethod::Locked.
public fun new_locked(): NotarizationMethod {
    NotarizationMethod::Locked
}

/// Returns true if the NotarizationMethod is Dynamic
public fun is_dynamic(method: &NotarizationMethod): bool {
    match (method) {
        NotarizationMethod::Dynamic => true,
        NotarizationMethod::Locked => false,
    }
}

/// Returns true if the NotarizationMethod is Locked
public fun is_locked(method: &NotarizationMethod): bool {
    match (method) {
        NotarizationMethod::Dynamic => false,
        NotarizationMethod::Locked => true,
    }
}

/// Returns the Notarization method as String
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
