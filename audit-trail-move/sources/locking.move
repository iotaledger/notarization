// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Locking configuration for audit trail records
module audit_trail::locking;

/// Defines a locking window (time XOR count based, or none)
public enum LockingWindow has copy, drop, store {
    None,
    TimeBased { seconds: u64 },
    CountBased { count: u64 },
}

/// Top-level locking configuration for the audit trail
public struct LockingConfig has copy, drop, store {
    /// Locking rules for record deletion
    delete_record: LockingWindow,
}

// ===== LockingWindow Constructors =====

/// Create a locking window with no restrictions
public fun window_none(): LockingWindow {
    LockingWindow::None
}

/// Create a time-based locking window
public fun window_time_based(seconds: u64): LockingWindow {
    LockingWindow::TimeBased { seconds }
}

/// Create a count-based locking window
public fun window_count_based(count: u64): LockingWindow {
    LockingWindow::CountBased { count }
}

// ===== LockingConfig Constructors =====

/// Create a new locking configuration
public fun new(delete_record: LockingWindow): LockingConfig {
    LockingConfig { delete_record }
}

// ===== LockingWindow Getters =====

/// Get the time window in seconds (if set)
public fun time_window_seconds(window: &LockingWindow): Option<u64> {
    match (window) {
        LockingWindow::TimeBased { seconds } => option::some(*seconds),
        _ => option::none(),
    }
}

/// Get the count window (if set)
public fun count_window(window: &LockingWindow): Option<u64> {
    match (window) {
        LockingWindow::CountBased { count } => option::some(*count),
        _ => option::none(),
    }
}

// ===== LockingConfig Getters =====

/// Get the record deletion locking window
public fun delete_record(config: &LockingConfig): &LockingWindow {
    &config.delete_record
}

// ===== LockingConfig Setters =====

/// Set the record deletion locking window
public(package) fun set_delete_record(config: &mut LockingConfig, window: LockingWindow) {
    config.delete_record = window;
}

// ===== Locking Logic (LockingWindow) =====

/// Check if a record is locked based on time window
///
/// Returns true if the record was created within the time window
public fun is_time_locked(window: &LockingWindow, record_timestamp: u64, current_time: u64): bool {
    match (window) {
        LockingWindow::TimeBased { seconds } => {
            let time_window_ms = (*seconds) * 1000;
            let record_age = current_time - record_timestamp;
            record_age < time_window_ms
        },
        _ => false,
    }
}

/// Check if a record is locked based on count window
///
/// Returns true if the record is among the last N records
public fun is_count_locked(window: &LockingWindow, sequence_number: u64, total_records: u64): bool {
    match (window) {
        LockingWindow::CountBased { count } => {
            let records_after = total_records - sequence_number - 1;
            records_after < *count
        },
        _ => false,
    }
}

/// Check if a record is locked by a window (either by time or count)
public fun is_window_locked(
    window: &LockingWindow,
    sequence_number: u64,
    record_timestamp: u64,
    total_records: u64,
    current_time: u64,
): bool {
    is_time_locked(window, record_timestamp, current_time)
        || is_count_locked(window, sequence_number, total_records)
}

// ===== Locking Logic (LockingConfig) =====

/// Check if a record is locked for deletion
public fun is_locked(
    config: &LockingConfig,
    sequence_number: u64,
    record_timestamp: u64,
    total_records: u64,
    current_time: u64,
): bool {
    is_window_locked(
        &config.delete_record,
        sequence_number,
        record_timestamp,
        total_records,
        current_time,
    )
}
