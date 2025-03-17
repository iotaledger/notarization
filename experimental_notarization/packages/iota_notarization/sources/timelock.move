// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// # Timelock Unlock Condition Module
///
/// This module implements a timelock mechanism that restricts access to resources
/// until a specified time has passed. It provides functionality to create and validate
/// different types of time-based locks:
///
/// - Simple time locks that unlock at a specific Unix timestamp
/// - Infinite locks that never unlock.
module iota_notarization::timelock {
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
        /// A permanent lock that never unlocks (Only used in State Locking)
        InfiniteLock
    }

    /// Creates a new time lock that unlocks at a specific Unix timestamp.
    public fun new_unlock_at(unix_time: u32, clock: &Clock): TimeLock {
        let now = (clock::timestamp_ms(clock) / 1000) as u32;

        assert!(is_valid_period(unix_time, now), EPastTimestamp);

        TimeLock::UnlockAt(unix_time)
    }

    /// Creates a new infinite lock that never unlocks.
    public fun infinite_lock(): TimeLock {
        TimeLock::InfiniteLock
    }

    /// Checks if the provided lock time is an infinite lock.
    public fun is_infinite_lock(lock_time: &TimeLock): bool {
        match (lock_time) {
            TimeLock::InfiniteLock => true,
            _ => false
        }
    }

    /// Destroys a TimeLock if it's either unlocked or an infinite lock.
    public fun destroy_if_unlocked_or_infinite_lock(condition: TimeLock, clock: &Clock) {
        match (condition) {
            TimeLock::UnlockAt(time) => {
                assert!(!(time > ((clock::timestamp_ms(clock) / 1000) as u32)), ETimelockNotExpired);
            },
            TimeLock::InfiniteLock => {}
        }
    }

    /// Checks if a timelock condition is currently active (locked).
    ///
    /// This function evaluates whether a given TimeLock instance is currently in a locked state
    /// by comparing the current time with the lock's parameters. A lock is considered active if:
    /// 1. For UnixTime locks: The current time hasn't reached the specified unlock time yet
    /// 2. For InfiniteLock: Always returns true as these locks never unlock
    public fun is_timelocked(condition: &TimeLock, clock: &Clock): bool {
        match (condition) {
            TimeLock::UnlockAt(unix_time) => {
                *unix_time > ((clock::timestamp_ms(clock) / 1000) as u32)
            },
            TimeLock::InfiniteLock => true
        }
    }

    /// Validates that a specified unlock time is in the future.
    public fun is_valid_period(unix_time: u32, current_time: u32): bool {
        unix_time > current_time
    }
}
