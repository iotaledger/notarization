// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides tests for the timelock module
#[test_only]
module iota_notarization::timelock_tests;

use iota::{clock, test_scenario::{Self as ts, ctx}};
use iota_notarization::timelock;

const ADMIN_ADDRESS: address = @0x01;

#[test]
public fun test_new_unlock_at() {
    let mut ts = ts::begin(ADMIN_ADDRESS);

    let ctx = ts.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    let lock = timelock::unlock_at(1001, &clock);

    assert!(timelock::is_unlock_at(&lock));
    assert!(timelock::get_unlock_time(&lock) == std::option::some(1001));
    assert!(timelock::is_timelocked(&lock, &clock));

    // Advance time by setting a new timestamp
    clock::increment_for_testing(&mut clock, 1000);

    assert!(!timelock::is_timelocked(&lock, &clock));

    timelock::destroy(lock, &clock);
    clock::destroy_for_testing(clock);

    ts.end();
}

#[test]
#[expected_failure(abort_code = timelock::EPastTimestamp)]
public fun test_new_unlock_at_past_time() {
    let mut ts = ts::begin(ADMIN_ADDRESS);
    let ctx = ts.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Try to create a timelock with a timestamp in the past
    let lock = timelock::unlock_at(999, &clock);

    // This should never be reached
    timelock::destroy(lock, &clock);
    clock::destroy_for_testing(clock);

    ts.end();
}

#[test]
public fun test_until_destroyed() {
    let mut ts = ts::begin(ADMIN_ADDRESS);
    let ctx = ts.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    let lock = timelock::until_destroyed();

    assert!(timelock::is_until_destroyed(&lock));
    assert!(!timelock::is_unlock_at(&lock));
    assert!(timelock::get_unlock_time(&lock) == std::option::none());

    // UntilDestroyed is always timelocked
    assert!(timelock::is_timelocked(&lock, &clock));

    // Even after a long time
    clock::increment_for_testing(&mut clock, 1000000);
    assert!(timelock::is_timelocked(&lock, &clock));

    // UntilDestroyed can always be destroyed without error
    timelock::destroy(lock, &clock);
    clock::destroy_for_testing(clock);

    ts.end();
}

#[test]
public fun test_none_lock() {
    let mut ts = ts::begin(ADMIN_ADDRESS);
    let ctx = ts.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    let lock = timelock::none();

    assert!(!timelock::is_until_destroyed(&lock));
    assert!(!timelock::is_unlock_at(&lock));
    assert!(timelock::get_unlock_time(&lock) == std::option::none());

    // None is never timelocked
    assert!(!timelock::is_timelocked(&lock, &clock));

    // None can always be destroyed without error
    timelock::destroy(lock, &clock);
    clock::destroy_for_testing(clock);

    ts.end();
}

#[test]
#[expected_failure(abort_code = timelock::ETimelockNotExpired)]
public fun test_destroy_locked_timelock() {
    let mut ts = ts::begin(ADMIN_ADDRESS);
    let ctx = ts.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a timelock that unlocks at time 2000
    let lock = timelock::unlock_at(2000, &clock);

    // Try to destroy it before it's unlocked
    // This should fail with ETimelockNotExpired
    timelock::destroy(lock, &clock);

    // These should never be reached
    clock::destroy_for_testing(clock);
    ts.end();
}

#[test]
public fun test_is_timelocked_unlock_at() {
    let mut ts = ts::begin(ADMIN_ADDRESS);
    let ctx = ts.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create different types of locks
    let unlock_at_lock = timelock::unlock_at(2000, &clock);
    let until_destroyed_lock = timelock::until_destroyed();
    let none_lock = timelock::none();

    // Test is_timelocked_unlock_at
    assert!(timelock::is_timelocked_unlock_at(&unlock_at_lock, &clock));
    assert!(!timelock::is_timelocked_unlock_at(&until_destroyed_lock, &clock));
    assert!(!timelock::is_timelocked_unlock_at(&none_lock, &clock));

    // Advance time past unlock time
    clock::increment_for_testing(&mut clock, 1000000);

    // Now the unlock_at lock should not be timelocked
    assert!(!timelock::is_timelocked_unlock_at(&unlock_at_lock, &clock));

    // Clean up
    timelock::destroy(unlock_at_lock, &clock);
    timelock::destroy(until_destroyed_lock, &clock);
    timelock::destroy(none_lock, &clock);
    clock::destroy_for_testing(clock);

    ts.end();
}

#[test]
public fun test_is_valid_period() {
    // Test valid periods
    assert!(timelock::is_valid_period(1001, 1000));
    assert!(timelock::is_valid_period(2000, 1000));

    // Test invalid periods
    assert!(!timelock::is_valid_period(1000, 1000)); // Equal time
    assert!(!timelock::is_valid_period(999, 1000)); // Past time
}

#[test]
public fun test_edge_cases() {
    let mut ts = ts::begin(ADMIN_ADDRESS);
    let ctx = ts.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Test with time just one second in the future
    let one_second_future = timelock::unlock_at(1001, &clock);
    assert!(timelock::is_timelocked(&one_second_future, &clock));
    clock::set_for_testing(&mut clock, 1001000);
    assert!(!timelock::is_timelocked(&one_second_future, &clock));

    // Test with time exactly at the current time boundary
    clock::set_for_testing(&mut clock, 2000000);
    let exact_current_time = timelock::unlock_at(2001, &clock);
    assert!(timelock::is_timelocked(&exact_current_time, &clock));
    clock::set_for_testing(&mut clock, 2001000);
    assert!(!timelock::is_timelocked(&exact_current_time, &clock));

    // Clean up
    timelock::destroy(one_second_future, &clock);
    timelock::destroy(exact_current_time, &clock);
    clock::destroy_for_testing(clock);

    ts.end();
}
