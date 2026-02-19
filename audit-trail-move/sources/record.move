// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Record module for audit trail entries
///
/// A Record represents a single entry in an audit trail, stored in a
/// LinkedTable and addressed by trail_id + sequence_number.
module audit_trail::record;

use iota::vec_set::{VecSet, Self};
use std::string::String;

/// A single record in the audit trail
public struct Record<D: store + copy> has store {
    /// Arbitrary data stored on-chain
    data: D,
    /// Optional metadata for this specific record
    metadata: Option<String>,
    /// Position in the trail (0-indexed, never reused)
    sequence_number: u64,
    /// Who added this record
    added_by: address,
    /// When this record was added (milliseconds)
    added_at: u64,
    /// Correction tracker for this record
    correction: RecordCorrection,
}

// ===== Constructors =====

/// Create a new record
public(package) fun new<D: store + copy>(
    data: D,
    metadata: Option<String>,
    sequence_number: u64,
    added_by: address,
    added_at: u64,
    correction: RecordCorrection,
): Record<D> {
    Record {
        data,
        metadata,
        sequence_number,
        added_by,
        added_at,
        correction,
    }
}

// ===== Getters =====

/// Get the stored data from a record
public fun data<D: store + copy>(record: &Record<D>): &D {
    &record.data
}

/// Get the record metadata
public fun metadata<D: store + copy>(record: &Record<D>): &Option<String> {
    &record.metadata
}

/// Get the record sequence number
public fun sequence_number<D: store + copy>(record: &Record<D>): u64 {
    record.sequence_number
}

/// Get who added the record
public fun added_by<D: store + copy>(record: &Record<D>): address {
    record.added_by
}

/// Get when the record was added (milliseconds)
public fun added_at<D: store + copy>(record: &Record<D>): u64 {
    record.added_at
}

/// Get the correction tracker for this record
public fun correction<D: store + copy>(record: &Record<D>): &RecordCorrection {
    &record.correction
}

/// Destroy a record
public(package) fun destroy<D: store + copy + drop>(record: Record<D>) {
    let Record {
        data: _,
        metadata: _,
        sequence_number: _,
        added_by: _,
        added_at: _,
        correction: _,
    } = record;
}


/// Bidirectional correction tracking for audit records
public struct RecordCorrection has copy, drop, store {
    replaces: VecSet<u64>,
    is_replaced_by: Option<u64>,
}

/// Create a new correction tracker for a normal (non-correcting) record
public fun new_correction(): RecordCorrection {
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
public(package) fun destroy_record_correction(correction: RecordCorrection) {
    let RecordCorrection { replaces: _, is_replaced_by: _ } = correction;
}
