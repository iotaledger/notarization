// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

module iota_notarization::timelock_unlock_condition {
    use iota::clock::{Self, Clock};

    // ===== Errors =====
    /// The timelock has not expired yet
    const ETimelockNotExpired: u64 = 3;
    /// Invalid timelock unlock condition provided
    const EInvalidTimelockUnlockCondition: u64 = 4;
    /// Infinite lock period
    const EInfiniteLockPeriod: u64 = 5;

    /// Special unix_time value that indicates infinite lock
    const INFINITE_LOCK: u32 = 0;

    /// The notarization timelock unlock condition.
    /// When unix_time is set to 0, it represents an infinite lock that never expires.
    public struct TimelockUnlockCondition has store {
        /// The unix time (seconds since Unix epoch) starting from which the condition unlocks.
        /// Special value 0 means infinitely locked.
        unix_time: u32
    }

    /// Create a new timelock condition
    /// # Arguments
    /// * `unix_time` - The unlock time in unix seconds. Use 0 for infinite lock.
    /// * `clock` - The clock instance to validate against current time
    public fun new(unix_time: u32, clock: &Clock): TimelockUnlockCondition {
        // Only validate non-infinite locks
        if (unix_time != INFINITE_LOCK) {
            let now = (clock::timestamp_ms(clock) / 1000) as u32;
            assert!(is_valid_period(unix_time, now), EInvalidTimelockUnlockCondition);
        };

        TimelockUnlockCondition { unix_time }
    }

    /// Check and consume the unlock condition.
    /// Aborts if the condition is still locked.
    public fun destroy_if_unlocked(condition: TimelockUnlockCondition, clock: &Clock) {
        assert!(is_infinite_lock(&condition), EInfiniteLockPeriod);
        assert!(!is_timelocked(&condition, clock), ETimelockNotExpired);

        let TimelockUnlockCondition {
            unix_time: _,
        } = condition;
    }

    /// Check if the condition is currently locked
    /// Returns true if either:
    /// 1. unix_time is 0 (infinite lock)
    /// 2. Current time hasn't reached unix_time yet
    public fun is_timelocked(condition: &TimelockUnlockCondition, clock: &Clock): bool {
        // Check for infinite lock first
        if (condition.unix_time == INFINITE_LOCK) {
            true
        } else {
            condition.unix_time > ((clock::timestamp_ms(clock) / 1000) as u32)
        }
    }

    /// Check if the provided unlock time is valid
    /// For non-infinite locks (unix_time != 0), the unlock time must be in the future
    public fun is_valid_period(unix_time: u32, current_time: u32): bool {
        unix_time == INFINITE_LOCK || unix_time > current_time
    }

    /// Get the unlock condition's unix time
    /// Returns 0 for infinite locks
    public fun unix_time(condition: &TimelockUnlockCondition): u32 {
        condition.unix_time
    }

    /// Check if this condition represents an infinite lock
    public fun is_infinite_lock(condition: &TimelockUnlockCondition): bool {
        condition.unix_time == INFINITE_LOCK
    }

    /// Returns the INFINITE_LOCK constant
    public fun infinite_lock() : u32 {
        INFINITE_LOCK
    }
}
