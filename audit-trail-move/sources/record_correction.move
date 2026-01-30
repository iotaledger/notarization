// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Module for tracking correction relationships for a record
module audit_trail::record_correction;

use iota::vec_set::{Self, VecSet};

/// Bidirectional correction tracking for audit records
public struct RecordCorrection has copy, drop, store {
    replaces: VecSet<u64>,
    is_replaced_by: Option<u64>,
}

/// Create a new correction tracker for a normal (non-correcting) record
public fun new(): RecordCorrection {
    RecordCorrection {
        replaces: vec_set::empty(),
        is_replaced_by: option::none(),
    }
}

/// Create a correction tracker for a correcting record
public fun with_replaces(replaced_seq_nums: VecSet<u64>): RecordCorrection {
    RecordCorrection {
        replaces: replaced_seq_nums,
        is_replaced_by: option::none(),
    }
}

/// Get the set of sequence numbers this record replaces
public fun replaces(correction: &RecordCorrection): &VecSet<u64> {
    &correction.replaces
}

/// Get the sequence number of the record that replaced this one
public fun is_replaced_by(correction: &RecordCorrection): Option<u64> {
    correction.is_replaced_by
}

/// Check if this record is a correction (replaces other records)
public fun is_correction(correction: &RecordCorrection): bool {
    !vec_set::is_empty(&correction.replaces)
}

/// Check if this record has been replaced by another record
public fun is_replaced(correction: &RecordCorrection): bool {
    correction.is_replaced_by.is_some()
}

/// Set the sequence number of the record that replaced this one
public(package) fun set_replaced_by(correction: &mut RecordCorrection, replacement_seq: u64) {
    correction.is_replaced_by = option::some(replacement_seq);
}

/// Add a sequence number to the set of records this record replaces
public(package) fun add_replaces(correction: &mut RecordCorrection, seq_num: u64) {
    correction.replaces.insert(seq_num);
}

/// Destroy a RecordCorrection
public(package) fun destroy(correction: RecordCorrection) {
    let RecordCorrection { replaces: _, is_replaced_by: _ } = correction;
}
