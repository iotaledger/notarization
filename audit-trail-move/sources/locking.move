// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Locking configuration for audit trail records
///
/// Controls when records can be deleted based on time window (records locked for N seconds)
/// or count window (last N records always locked).
module audit_trail::locking;

/// Controls when records can be deleted (time OR count based)
public struct LockingConfig has copy, drop, store {
    /// Records locked for N seconds after creation
    time_window_seconds: Option<u64>,
    /// Last N records are always locked
    count_window: Option<u64>,
}

// ===== Constructors =====

/// Create a new locking configuration
///
/// - `time_window_seconds`: Records are locked for N seconds after creation (None = no time lock)
/// - `count_window`: Last N records are always locked (None = no count lock)
public fun new(time_window_seconds: Option<u64>, count_window: Option<u64>): LockingConfig {
    LockingConfig { time_window_seconds, count_window }
}

/// Create a locking config with no restrictions
public fun none(): LockingConfig {
    LockingConfig {
        time_window_seconds: option::none(),
        count_window: option::none(),
    }
}

/// Create a time-based locking config
public fun time_based(seconds: u64): LockingConfig {
    LockingConfig {
        time_window_seconds: option::some(seconds),
        count_window: option::none(),
    }
}

/// Create a count-based locking config
public fun count_based(count: u64): LockingConfig {
    LockingConfig {
        time_window_seconds: option::none(),
        count_window: option::some(count),
    }
}

// ===== Getters =====

/// Get the time window in seconds (if set)
public fun time_window_seconds(config: &LockingConfig): &Option<u64> {
    &config.time_window_seconds
}

/// Get the count window (if set)
public fun count_window(config: &LockingConfig): &Option<u64> {
    &config.count_window
}

// ===== Locking Logic =====

/// Check if a record is locked based on time window
///
/// Returns true if the record was created within the time window
public fun is_time_locked(config: &LockingConfig, record_timestamp: u64, current_time: u64): bool {
    if (config.time_window_seconds.is_none()) {
        return false
    };

    let time_window_ms = (*config.time_window_seconds.borrow()) * 1000;
    let record_age = current_time - record_timestamp;
    record_age < time_window_ms
}

/// Check if a record is locked based on count window
///
/// Returns true if the record is among the last N records
public fun is_count_locked(config: &LockingConfig, sequence_number: u64, total_records: u64): bool {
    if (config.count_window.is_none()) {
        return false
    };

    let count_window = *config.count_window.borrow();

    let records_after = total_records - sequence_number - 1;
    records_after < count_window
}

/// Check if a record is locked (either by time or count)
public fun is_locked(
    config: &LockingConfig,
    sequence_number: u64,
    record_timestamp: u64,
    total_records: u64,
    current_time: u64,
): bool {
    is_time_locked(config, record_timestamp, current_time)
        || is_count_locked(config, sequence_number, total_records)
}
