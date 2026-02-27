// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Locking configuration for audit trail records
module audit_trail::locking;

use iota::clock::Clock;
use tf_components::timelock::{Self, TimeLock};

// ===== Errors =====

/// UntilDestroyed cannot be used for trail deletion protection.
const EUntilDestroyedNotSupportedForDeleteTrail: u64 = 0;

/// Defines a locking window (time XOR count based, or none)
public enum LockingWindow has copy, drop, store {
    None,
    TimeBased { seconds: u64 },
    CountBased { count: u64 },
}

/// Top-level locking configuration for the audit trail
public struct LockingConfig has drop, store {
    /// Locking rules for record deletion
    delete_record_window: LockingWindow,
    /// Timelock protecting deletion of the trail itself
    delete_trail_lock: TimeLock,
    /// Timelock protecting record writes (add_record)
    write_lock: TimeLock,
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
public fun new(
    delete_record_window: LockingWindow,
    delete_trail_lock: TimeLock,
    write_lock: TimeLock,
): LockingConfig {
    assert!(
        !timelock::is_until_destroyed(&delete_trail_lock),
        EUntilDestroyedNotSupportedForDeleteTrail,
    );

    LockingConfig {
        delete_record_window,
        delete_trail_lock,
        write_lock,
    }
}

// ===== LockingWindow Getters =====

/// Get the time window in seconds (if set)
public(package) fun time_window_seconds(window: &LockingWindow): Option<u64> {
    match (window) {
        LockingWindow::TimeBased { seconds } => option::some(*seconds),
        _ => option::none(),
    }
}

/// Get the count window (if set)
public(package) fun count_window(window: &LockingWindow): Option<u64> {
    match (window) {
        LockingWindow::CountBased { count } => option::some(*count),
        _ => option::none(),
    }
}

// ===== LockingConfig Getters =====

/// Get the record deletion locking window
public(package) fun delete_record_window(config: &LockingConfig): &LockingWindow {
    &config.delete_record_window
}

/// Get the trail deletion timelock
public(package) fun delete_trail_lock(config: &LockingConfig): &TimeLock {
    &config.delete_trail_lock
}

/// Get the write timelock
public(package) fun write_lock(config: &LockingConfig): &TimeLock {
    &config.write_lock
}

// ===== LockingConfig Setters =====

/// Set the record deletion locking window
public(package) fun set_delete_record_window(config: &mut LockingConfig, window: LockingWindow) {
    config.delete_record_window = window;
}

/// Set the trail deletion timelock.
public(package) fun set_delete_trail_lock(config: &mut LockingConfig, lock: TimeLock) {
    assert!(
        !timelock::is_until_destroyed(&lock),
        EUntilDestroyedNotSupportedForDeleteTrail,
    );

    config.delete_trail_lock = lock;
}

/// Set the write timelock.
public(package) fun set_write_lock(config: &mut LockingConfig, lock: TimeLock) {
    config.write_lock = lock;
}

/// Set the whole locking configuration.
public(package) fun set_config(config: &mut LockingConfig, new_config: LockingConfig) {
    let LockingConfig {
        delete_record_window,
        delete_trail_lock,
        write_lock,
    } = new_config;

    set_delete_record_window(config, delete_record_window);
    set_delete_trail_lock(config, delete_trail_lock);
    set_write_lock(config, write_lock);
}

// ===== Locking Logic (LockingWindow) =====

/// Check if a record is locked based on time window.
/// Returns true if the record was created within the time window.
fun is_time_locked(window: &LockingWindow, record_timestamp: u64, current_time: u64): bool {
    match (window) {
        LockingWindow::TimeBased { seconds } => {
            let time_window_ms = (*seconds) * 1000;
            let record_age = current_time - record_timestamp;
            record_age < time_window_ms
        },
        _ => false,
    }
}

/// Check if a record is locked based on count window.
/// Returns true if the record is among the last N records.
fun is_count_locked(window: &LockingWindow, sequence_number: u64, total_records: u64): bool {
    match (window) {
        LockingWindow::CountBased { count } => {
            let records_after = total_records - sequence_number - 1;
            records_after < *count
        },
        _ => false,
    }
}

/// Check if a record is locked by a window (either by time or count).
fun is_window_locked(
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

/// Check if a record is locked for deletion.
public fun is_delete_record_locked(
    config: &LockingConfig,
    sequence_number: u64,
    record_timestamp: u64,
    total_records: u64,
    current_time: u64,
): bool {
    is_window_locked(
        &config.delete_record_window,
        sequence_number,
        record_timestamp,
        total_records,
        current_time,
    )
}

/// Check if trail deletion is currently locked.
public fun is_delete_trail_locked(config: &LockingConfig, clock: &Clock): bool {
    timelock::is_timelocked(delete_trail_lock(config), clock)
}

/// Check if writes are currently locked.
public fun is_write_locked(config: &LockingConfig, clock: &Clock): bool {
    timelock::is_timelocked(write_lock(config), clock)
}
