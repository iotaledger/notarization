// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides tests for the locked_notarization module
#[test_only]
module iota_notarization::locked_notarization_tests;

use iota::{clock, test_scenario as ts};
use iota_notarization::{locked_notarization, notarization, timelock};
use std::string;

const ADMIN_ADDRESS: address = @0x1;

#[test]
public fun test_create_locked_notarization() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state with string data
    let data = string::utf8(b"Test Data");
    let metadata = std::option::some(string::utf8(b"Test Metadata"));
    let state = notarization::new_state_from_string(data, metadata);

    // Create a delete lock that unlocks at timestamp 2000
    let delete_lock = timelock::unlock_at(2000, &clock);

    // Create a locked notarization
    locked_notarization::create(
        state,
        std::option::some(string::utf8(b"Test Description")),
        std::option::some(string::utf8(b"Test Updatable Metadata")),
        delete_lock,
        &clock,
        ctx,
    );

    ts::next_tx(&mut scenario, ADMIN_ADDRESS);

    // Check that the notarization was created and transferred to ADMIN_ADDRESS
    let notarization = ts::take_from_sender<notarization::Notarization<string::String>>(
        &scenario,
    );

    // Verify notarization properties
    assert!(notarization::notarization_method(&notarization).is_locked(), 0);
    assert!(
        notarization::description(&notarization) == &std::option::some(string::utf8(b"Test Description")),
        0,
    );
    assert!(
        notarization::updatable_metadata(&notarization) == &std::option::some(string::utf8(b"Test Updatable Metadata")),
        0,
    );
    assert!(notarization::created_at(&notarization) == 1000000, 0);
    assert!(notarization::version_count(&notarization) == 0, 0);

    // Verify we can't destroy it before the lock expires
    assert!(notarization::is_delete_locked(&notarization, &clock), 0);

    // Advance time past the lock
    clock::increment_for_testing(&mut clock, 1000000 + 1);

    // Now we should be able to destroy it
    assert!(!notarization::is_delete_locked(&notarization, &clock), 0);

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = notarization::EUntilDestroyedLockNotAllowed)]
public fun test_create_locked_notarization_with_until_destroyed_delete_lock() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state
    let data = string::utf8(b"Test Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a delete lock that is locked until destroyed
    let delete_lock = timelock::until_destroyed();

    // Create a locked notarization
    locked_notarization::create(
        state,
        std::option::none(),
        std::option::none(),
        delete_lock,
        &clock,
        ctx,
    );

    ts::next_tx(&mut scenario, ADMIN_ADDRESS);

    // Check that the notarization was created and transferred to ADMIN_ADDRESS
    let notarization = ts::take_from_sender<notarization::Notarization<string::String>>(
        &scenario,
    );

    // Verify it's not destroyable (has until_destroyed delete lock)
    assert!(notarization::is_delete_locked(&notarization, &clock), 0);

    // Advance time - should still be locked
    clock::increment_for_testing(&mut clock, 1000000);
    assert!(notarization::is_delete_locked(&notarization, &clock), 0);

    // Clean up - this should fail if uncommented due to until_destroyed lock
    notarization::destroy(notarization, &clock);

    // Instead, just clean up the clock since we can't destroy the notarization
    clock::destroy_for_testing(clock);

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = notarization::EUpdateWhileLocked)]
public fun test_update_locked_notarization() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state
    let data = string::utf8(b"Initial Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a delete lock
    let delete_lock = timelock::unlock_at(2000, &clock);

    // Create a locked notarization
    locked_notarization::create(
        state,
        std::option::none(),
        std::option::none(),
        delete_lock,
        &clock,
        ctx,
    );

    ts::next_tx(&mut scenario, ADMIN_ADDRESS);

    // Take the notarization and update its state
    let mut notarization = ts::take_from_sender<notarization::Notarization<string::String>>(
        &scenario,
    );

    // Create new state
    let new_data = string::utf8(b"Updated Data");
    let new_metadata = std::option::some(string::utf8(b"Updated Metadata"));
    let new_state = notarization::new_state_from_string(new_data, new_metadata);

    // Update the state (Should fail)
    notarization::update_state(&mut notarization, new_state, &clock);

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = notarization::EDestroyWhileLocked)]
public fun test_destroy_locked_notarization_before_unlock() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = scenario.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state
    let data = string::utf8(b"Test Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a delete lock that unlocks at timestamp 2000
    let delete_lock = timelock::unlock_at(2000, &clock);

    // Create a locked notarization
    locked_notarization::create(
        state,
        std::option::none(),
        std::option::none(),
        delete_lock,
        &clock,
        ctx,
    );

    ts::next_tx(&mut scenario, ADMIN_ADDRESS);

    // Take the notarization
    let notarization = ts::take_from_sender<notarization::Notarization<string::String>>(
        &scenario,
    );

    // Try to destroy before lock expires - should fail
    notarization::destroy(notarization, &clock);

    // Clean up - we won't reach this due to expected failure
    clock::destroy_for_testing(clock);
    ts::end(scenario);
}

#[test]
public fun test_locked_notarization_with_none_lock() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = scenario.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state
    let data = string::utf8(b"Test Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a delete lock that is never locked
    let delete_lock = timelock::none();

    // Create a locked notarization
    locked_notarization::create(
        state,
        std::option::none(),
        std::option::none(),
        delete_lock,
        &clock,
        ctx,
    );

    ts::next_tx(&mut scenario, ADMIN_ADDRESS);

    // Check that the notarization was created and transferred to ADMIN_ADDRESS
    let notarization = ts::take_from_sender<notarization::Notarization<string::String>>(
        &scenario,
    );

    // Verify it's destroyable (has none delete lock)
    assert!(!notarization::is_delete_locked(&notarization, &clock), 0);

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    ts::end(scenario);
}
