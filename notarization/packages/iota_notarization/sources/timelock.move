// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// # Timelock Unlock Condition Module
///
/// This module implements a timelock mechanism that restricts access to resources
/// until a specified time has passed. It provides functionality to create and validate
/// different types of time-based locks:
///
/// - Simple time locks that unlock at a specific Unix timestamp
/// - UntilDestroyed lock that never unlocks until the notarization is destroyed
/// - None lock that is not locked
module iota_notarization::timelock;

use iota::clock::{Self, Clock};

// ===== Errors =====
/// Error when attempting to create a timelock with a timestamp in the past
const EPastTimestamp: u64 = 0;
/// Error when attempting to destroy a timelock that is still locked
const ETimelockNotExpired: u64 = 1;

/// Represents different types of time-based locks that can be applied to
/// notarizations.
public enum TimeLock has store {
    /// A lock that unlocks at a specific Unix timestamp (seconds since epoch)
    UnlockAt(u32),
    /// A permanent lock that never unlocks (can't be used for locking delete)
    UntilDestroyed,
    /// No lock applied
    None,
}

/// Creates a new time lock that unlocks at a specific Unix timestamp.
public fun unlock_at(unix_time: u32, clock: &Clock): TimeLock {
    let now = (clock::timestamp_ms(clock) / 1000) as u32;

    assert!(is_valid_period(unix_time, now), EPastTimestamp);

    TimeLock::UnlockAt(unix_time)
}

/// Creates a new UntilDestroyed lock that never unlocks.
public fun until_destroyed(): TimeLock {
    TimeLock::UntilDestroyed
}

/// Create a new lock that is not locked.
public fun none(): TimeLock {
    TimeLock::None
}

/// Checks if the provided lock time is an UntilDestroyed lock.
public fun is_until_destroyed(lock_time: &TimeLock): bool {
    match (lock_time) {
        TimeLock::UntilDestroyed => true,
        _ => false,
    }
}

/// Checks if the provided lock time is a UnlockAt lock.
public fun is_unlock_at(lock_time: &TimeLock): bool {
    match (lock_time) {
        TimeLock::UnlockAt(_) => true,
        _ => false,
    }
}

/// Checks if the provided lock time is a None lock.
public fun is_none(lock_time: &TimeLock): bool {
    match (lock_time) {
        TimeLock::None => true,
        _ => false,
    }
}

/// Gets the unlock time from a TimeLock if it is a UnixTime lock.
public fun get_unlock_time(lock_time: &TimeLock): Option<u32> {
    match (lock_time) {
        TimeLock::UnlockAt(time) => option::some(*time),
        _ => option::none(),
    }
}

/// Destroys a TimeLock if it's either unlocked or an UntilDestroyed lock.
public fun destroy(condition: TimeLock, clock: &Clock) {
    // The TimeLock is always destroyed, except of those cases where an assertion is raised
    match (condition) {
        TimeLock::UnlockAt(time) => {
            assert!(!(time > ((clock::timestamp_ms(clock) / 1000) as u32)), ETimelockNotExpired);
        },
        TimeLock::UntilDestroyed => {},
        TimeLock::None => {},
    }
}

/// Checks if a timelock condition is currently active (locked).
///
/// This function evaluates whether a given TimeLock instance is currently in a locked state
/// by comparing the current time with the lock's parameters. A lock is considered active if:
/// 1. For UnixTime locks: The current time hasn't reached the specified unlock time yet
/// 2. For UntilDestroyed: Always returns true as these locks never unlock until the notarization is destroyed
/// 3. For None: Always returns false as there is no lock
public fun is_timelocked(condition: &TimeLock, clock: &Clock): bool {
    match (condition) {
        TimeLock::UnlockAt(unix_time) => {
            *unix_time > ((clock::timestamp_ms(clock) / 1000) as u32)
        },
        TimeLock::UntilDestroyed => true,
        TimeLock::None => false,
    }
}

/// Check if a timelock condition is `UnlockAt`
public fun is_timelocked_unlock_at(lock_time: &TimeLock, clock: &Clock): bool {
    match (lock_time) {
        TimeLock::UnlockAt(time) => {
            *time > ((clock::timestamp_ms(clock) / 1000) as u32)
        },
        _ => false,
    }
}

/// Validates that a specified unlock time is in the future.
public fun is_valid_period(unix_time: u32, current_time: u32): bool {
    unix_time > current_time
}
