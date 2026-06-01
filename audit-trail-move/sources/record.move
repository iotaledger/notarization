// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Record module for audit trail entries
///
/// A Record represents a single entry in an audit trail, stored in a
/// LinkedTable and addressed by trail_id + sequence_number.
module audit_trail::record;

use iota::vec_set::{Self, VecSet};
use std::string::String;

/// Flexible record payload that can store either raw bytes or text.
public enum Data has copy, drop, store {
    Bytes(vector<u8>),
    Text(String),
}

/// Creates a `Data` value carrying the given byte payload.
///
/// Returns the `Data::Bytes` variant.
public fun new_bytes(bytes: vector<u8>): Data {
    Data::Bytes(bytes)
}

/// Creates a `Data` value carrying the given text payload.
///
/// Returns the `Data::Text` variant.
public fun new_text(text: String): Data {
    Data::Text(text)
}

/// Extracts the byte payload from a `Data` value when present.
///
/// Returns `option::some(bytes)` when `data` is `Data::Bytes`, otherwise
/// `option::none()`.
public fun bytes(data: &Data): Option<vector<u8>> {
    match (data) {
        Data::Bytes(bytes) => option::some(*bytes),
        Data::Text(_) => option::none(),
    }
}

/// Extracts the text payload from a `Data` value when present.
///
/// Returns `option::some(text)` when `data` is `Data::Text`, otherwise
/// `option::none()`.
public fun text(data: &Data): Option<String> {
    match (data) {
        Data::Bytes(_) => option::none(),
        Data::Text(text) => option::some(*text),
    }
}

/// A single record in the audit trail
public struct Record<D: store + copy> has store {
    /// Arbitrary data stored on-chain
    data: D,
    /// Optional metadata for this specific record
    metadata: Option<String>,
    /// Optional immutable tag associated with this record
    tag: Option<String>,
    /// Position in the trail (0-indexed, never reused)
    sequence_number: u64,
    /// Who added this record
    added_by: address,
    /// When this record was added (milliseconds)
    added_at: u64,
    /// Correction tracker for this record
    correction: RecordCorrection,
}

/// Input used when creating a trail with an initial record.
public struct InitialRecord<D: store + copy> has copy, drop, store {
    data: D,
    metadata: Option<String>,
    tag: Option<String>,
}

// ===== Constructors =====

/// Creates an `InitialRecord` to be passed to `audit_trail::create`.
///
/// Returns the constructed `InitialRecord`.
public fun new_initial_record<D: store + copy>(
    data: D,
    metadata: Option<String>,
    tag: Option<String>,
): InitialRecord<D> {
    InitialRecord { data, metadata, tag }
}

/// Creates a new `Record` from its constituent fields.
///
/// Returns the constructed `Record`.
public(package) fun new<D: store + copy>(
    data: D,
    metadata: Option<String>,
    tag: Option<String>,
    sequence_number: u64,
    added_by: address,
    added_at: u64,
    correction: RecordCorrection,
): Record<D> {
    Record {
        data,
        metadata,
        tag,
        sequence_number,
        added_by,
        added_at,
        correction,
    }
}

/// Converts an `InitialRecord` into a stored `Record` with an empty correction tracker.
///
/// Returns the resulting `Record` ready to be inserted into the trail's record table.
public(package) fun into_record<D: store + copy>(
    initial_record: InitialRecord<D>,
    sequence_number: u64,
    added_by: address,
    added_at: u64,
): Record<D> {
    let InitialRecord { data, metadata, tag } = initial_record;
    new(
        data,
        metadata,
        tag,
        sequence_number,
        added_by,
        added_at,
        empty(),
    )
}

// ===== Getters =====

/// Returns a reference to the data payload stored in the record.
public fun data<D: store + copy>(self: &Record<D>): &D {
    &self.data
}

/// Returns a reference to the record's optional metadata field.
public fun metadata<D: store + copy>(self: &Record<D>): &Option<String> {
    &self.metadata
}

/// Returns a reference to the record's optional tag field.
public fun tag<D: store + copy>(record: &Record<D>): &Option<String> {
    &record.tag
}

/// Returns the record's position in the trail (zero-indexed sequence number).
public fun sequence_number<D: store + copy>(self: &Record<D>): u64 {
    self.sequence_number
}

/// Returns the address that added the record to the trail.
public fun added_by<D: store + copy>(self: &Record<D>): address {
    self.added_by
}

/// Returns the record's creation timestamp in milliseconds since the Unix epoch.
public fun added_at<D: store + copy>(self: &Record<D>): u64 {
    self.added_at
}

/// Returns a reference to the record's bidirectional correction tracker.
public fun correction<D: store + copy>(self: &Record<D>): &RecordCorrection {
    &self.correction
}

/// Destroys a `Record` by destructuring it.
public(package) fun destroy<D: store + copy + drop>(self: Record<D>) {
    let Record {
        data: _,
        metadata: _,
        tag: _,
        sequence_number: _,
        added_by: _,
        added_at: _,
        correction: _,
    } = self;
}

/// Bidirectional correction tracking for audit records
public struct RecordCorrection has copy, drop, store {
    replaces: VecSet<u64>,
    is_replaced_by: Option<u64>,
}

/// Creates an empty correction tracker for a record that does not correct any other.
///
/// Returns a `RecordCorrection` with an empty `replaces` set and no
/// `is_replaced_by` reference.
public fun empty(): RecordCorrection {
    RecordCorrection {
        replaces: vec_set::empty(),
        is_replaced_by: option::none(),
    }
}

/// Creates a correction tracker for a record that replaces other records.
///
/// Returns a `RecordCorrection` whose `replaces` set is `replaced_seq_nums` and
/// whose `is_replaced_by` is unset.
public fun with_replaces(replaced_seq_nums: VecSet<u64>): RecordCorrection {
    RecordCorrection {
        replaces: replaced_seq_nums,
        is_replaced_by: option::none(),
    }
}

/// Returns a reference to the set of sequence numbers this record replaces.
public fun replaces(correction: &RecordCorrection): &VecSet<u64> {
    &correction.replaces
}

/// Returns the sequence number of the record that replaced this one, when any.
public fun is_replaced_by(correction: &RecordCorrection): Option<u64> {
    correction.is_replaced_by
}

/// Checks whether this record corrects (replaces) at least one other record.
///
/// Returns `true` when `replaces` is non-empty.
public fun is_correction(correction: &RecordCorrection): bool {
    !vec_set::is_empty(&correction.replaces)
}

/// Checks whether this record has been replaced by another record.
///
/// Returns `true` when `is_replaced_by` is set.
public fun is_replaced(correction: &RecordCorrection): bool {
    correction.is_replaced_by.is_some()
}

/// Records that this record has been replaced by `replacement_seq`.
public(package) fun set_replaced_by(correction: &mut RecordCorrection, replacement_seq: u64) {
    correction.is_replaced_by = option::some(replacement_seq);
}

/// Adds `seq_num` to the set of records this record replaces.
public(package) fun add_replaces(correction: &mut RecordCorrection, seq_num: u64) {
    correction.replaces.insert(seq_num);
}

/// Destroys a `RecordCorrection` by destructuring it.
public(package) fun destroy_record_correction(correction: RecordCorrection) {
    let RecordCorrection { replaces: _, is_replaced_by: _ } = correction;
}
