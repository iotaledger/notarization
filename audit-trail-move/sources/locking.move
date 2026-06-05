// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Locking configuration for audit trail records
module audit_trails::locking;

use iota::clock::Clock;
use tf_components::timelock::{Self, TimeLock};

// ===== Errors =====

/// UntilDestroyed cannot be used for trail deletion protection.
const EUntilDestroyedNotSupportedForDeleteTrail: u64 = 0;

/// A count-based locking window must protect at least one record.
const ECountWindowMustBePositive: u64 = 1;

/// Defines a delete-record locking window (time-based, count-based, or none).
///
/// A window describes the period during which a record stays locked against
/// deletion. Records outside the window may be deleted, subject to remaining
/// permission and tag checks.
public enum LockingWindow has copy, drop, store {
    None,
    TimeBased { seconds: u64 },
    /// Locks the last `count` records currently present in trail order.
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

/// Creates a locking window that imposes no time- or count-based restrictions.
///
/// Returns the `LockingWindow::None` variant.
public fun window_none(): LockingWindow {
    LockingWindow::None
}

/// Creates a time-based locking window.
///
/// Records that were added less than `seconds` seconds ago are considered locked.
///
/// Returns the `LockingWindow::TimeBased` variant.
public fun window_time_based(seconds: u64): LockingWindow {
    LockingWindow::TimeBased { seconds }
}

/// Creates a count-based locking window.
///
/// The trail locks the last `count` records currently present in linked-table
/// order. To express "no deletion lock", use `window_none()` instead of
/// passing `count == 0`.
///
/// Aborts
/// ------
/// * `ECountWindowMustBePositive` if `count == 0`. A zero-count window would
///   protect no records and is functionally identical to `window_none()`;
///   rejecting it at construction prevents silently misconfigured trails.
///
/// Returns the `LockingWindow::CountBased` variant.
public fun window_count_based(count: u64): LockingWindow {
    assert!(count > 0, ECountWindowMustBePositive);
    LockingWindow::CountBased { count }
}

// ===== LockingConfig Constructors =====

/// Creates a new locking configuration.
///
/// `TimeLock::UntilDestroyed` is reserved for the write lock and is not accepted as
/// `delete_trail_lock`.
///
/// Aborts with:
/// * `EUntilDestroyedNotSupportedForDeleteTrail` when `delete_trail_lock` is
///   `TimeLock::UntilDestroyed`.
///
/// Returns the constructed `LockingConfig`.
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

/// Returns the time window in seconds when `window` is `LockingWindow::TimeBased`,
/// otherwise `option::none()`.
public(package) fun time_window_seconds(window: &LockingWindow): Option<u64> {
    match (window) {
        LockingWindow::TimeBased { seconds } => option::some(*seconds),
        _ => option::none(),
    }
}

/// Returns the count window when `window` is `LockingWindow::CountBased`,
/// otherwise `option::none()`.
public(package) fun count_window(window: &LockingWindow): Option<u64> {
    match (window) {
        LockingWindow::CountBased { count } => option::some(*count),
        _ => option::none(),
    }
}

// ===== LockingConfig Getters =====

/// Returns a reference to the configuration's record-deletion locking window.
public(package) fun delete_record_window(config: &LockingConfig): &LockingWindow {
    &config.delete_record_window
}

/// Returns a reference to the configuration's trail-deletion timelock.
public(package) fun delete_trail_lock(config: &LockingConfig): &TimeLock {
    &config.delete_trail_lock
}

/// Returns a reference to the configuration's write timelock.
public(package) fun write_lock(config: &LockingConfig): &TimeLock {
    &config.write_lock
}

// ===== LockingConfig Setters =====

/// Sets the configuration's record-deletion locking window to `window`.
public(package) fun set_delete_record_window(config: &mut LockingConfig, window: LockingWindow) {
    config.delete_record_window = window;
}

/// Sets the configuration's trail-deletion timelock to `lock`.
///
/// `TimeLock::UntilDestroyed` is reserved for the write lock and is not accepted here.
///
/// Aborts with:
/// * `EUntilDestroyedNotSupportedForDeleteTrail` when `lock` is
///   `TimeLock::UntilDestroyed`.
public(package) fun set_delete_trail_lock(config: &mut LockingConfig, lock: TimeLock) {
    assert!(!timelock::is_until_destroyed(&lock), EUntilDestroyedNotSupportedForDeleteTrail);

    config.delete_trail_lock = lock;
}

/// Sets the configuration's write timelock to `lock`.
public(package) fun set_write_lock(config: &mut LockingConfig, lock: TimeLock) {
    config.write_lock = lock;
}

/// Replaces the entire locking configuration with `new_config`.
///
/// Internally applies `set_delete_record_window`, `set_delete_trail_lock` and
/// `set_write_lock`, so the constraints documented for those setters apply.
///
/// Aborts with:
/// * `EUntilDestroyedNotSupportedForDeleteTrail` when
///   `new_config.delete_trail_lock` is `TimeLock::UntilDestroyed`.
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

/// Checks whether a record is locked by the time-based window.
///
/// Returns `true` when `window` is `LockingWindow::TimeBased` and the record's age
/// is below the configured number of seconds.
public(package) fun is_time_locked(
    window: &LockingWindow,
    record_timestamp: u64,
    current_time: u64,
): bool {
    match (window) {
        LockingWindow::TimeBased { seconds } => {
            let time_window_ms = (*seconds) * 1000;
            let record_age = current_time - record_timestamp;
            record_age < time_window_ms
        },
        _ => false,
    }
}

// ===== Locking Logic (LockingConfig) =====

/// Checks whether trail deletion is currently blocked by `delete_trail_lock`.
///
/// Returns `true` while the configured timelock has not yet elapsed.
public fun is_delete_trail_locked(config: &LockingConfig, clock: &Clock): bool {
    timelock::is_timelocked(delete_trail_lock(config), clock)
}

/// Checks whether record writes are currently blocked by `write_lock`.
///
/// Returns `true` while the configured timelock has not yet elapsed.
public fun is_write_locked(config: &LockingConfig, clock: &Clock): bool {
    timelock::is_timelocked(write_lock(config), clock)
}
