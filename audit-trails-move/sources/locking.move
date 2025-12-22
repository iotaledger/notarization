// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Locking configuration for audit trail records
module audit_trails::locking;

/// Defines a locking window (time OR count based)
public struct LockingWindow has copy, drop, store {
    /// Records locked for N seconds after creation
    time_window_seconds: Option<u64>,
    /// Last N records are always locked
    count_window: Option<u64>,
}

/// Top-level locking configuration for the audit trail
public struct LockingConfig has copy, drop, store {
    /// Locking rules for record deletion
    delete_record_lock: LockingWindow,
}

// ===== LockingWindow Constructors =====

/// Create a new locking window
///
/// - `time_window_seconds`: Records are locked for N seconds after creation (None = no time lock)
/// - `count_window`: Last N records are always locked (None = no count lock)
public fun new_window(time_window_seconds: Option<u64>, count_window: Option<u64>): LockingWindow {
    LockingWindow { time_window_seconds, count_window }
}

/// Create a locking window with no restrictions
public fun window_none(): LockingWindow {
    LockingWindow {
        time_window_seconds: option::none(),
        count_window: option::none(),
    }
}

/// Create a time-based locking window
public fun window_time_based(seconds: u64): LockingWindow {
    LockingWindow {
        time_window_seconds: option::some(seconds),
        count_window: option::none(),
    }
}

/// Create a count-based locking window
public fun window_count_based(count: u64): LockingWindow {
    LockingWindow {
        time_window_seconds: option::none(),
        count_window: option::some(count),
    }
}

// ===== LockingConfig Constructors =====

/// Create a new locking configuration
public fun new(delete_record_lock: LockingWindow): LockingConfig {
    LockingConfig { delete_record_lock }
}

/// Create a locking config with no restrictions
public fun none(): LockingConfig {
    LockingConfig {
        delete_record_lock: window_none(),
    }
}

/// Create a locking config with time-based record deletion lock
public fun time_based(seconds: u64): LockingConfig {
    LockingConfig {
        delete_record_lock: window_time_based(seconds),
    }
}

/// Create a locking config with count-based record deletion lock
public fun count_based(count: u64): LockingConfig {
    LockingConfig {
        delete_record_lock: window_count_based(count),
    }
}

// ===== LockingWindow Getters =====

/// Get the time window in seconds (if set)
public fun time_window_seconds(window: &LockingWindow): &Option<u64> {
    &window.time_window_seconds
}

/// Get the count window (if set)
public fun count_window(window: &LockingWindow): &Option<u64> {
    &window.count_window
}

// ===== LockingConfig Getters =====

/// Get the record deletion locking window
public fun delete_record_lock(config: &LockingConfig): &LockingWindow {
    &config.delete_record_lock
}

// ===== Locking Logic (LockingWindow) =====

/// Check if a record is locked based on time window
///
/// Returns true if the record was created within the time window
public fun is_time_locked(window: &LockingWindow, record_timestamp: u64, current_time: u64): bool {
    if (window.time_window_seconds.is_none()) {
        return false
    };

    let time_window_ms = (*window.time_window_seconds.borrow()) * 1000;
    let record_age = current_time - record_timestamp;
    record_age < time_window_ms
}

/// Check if a record is locked based on count window
///
/// Returns true if the record is among the last N records
public fun is_count_locked(window: &LockingWindow, sequence_number: u64, total_records: u64): bool {
    if (window.count_window.is_none()) {
        return false
    };

    let count_window = *window.count_window.borrow();

    let records_after = total_records - sequence_number - 1;
    records_after < count_window
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
        &config.delete_record_lock,
        sequence_number,
        record_timestamp,
        total_records,
        current_time,
    )
}
