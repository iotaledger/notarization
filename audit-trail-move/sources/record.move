// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Record module for audit trail entries
///
/// A Record represents a single entry in an audit trail, stored in a
/// LinkedTable and addressed by trail_id + sequence_number.
module audit_trail::record;

use audit_trail::record_correction::RecordCorrection;
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
