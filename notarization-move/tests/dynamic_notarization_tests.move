// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides tests for the dynamic_notarization module
#[test_only]
module iota_notarization::dynamic_notarization_tests;

use iota::{clock, test_scenario::{Self as ts, ctx}};
use iota_notarization::{dynamic_notarization, notarization, timelock};
use std::string;

const ADMIN_ADDRESS: address = @0x01;
const RECIPIENT_ADDRESS: address = @0x02;

#[test]
public fun test_create_dynamic_notarization_with_string_data() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = scenario.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state with string data
    let data = string::utf8(b"Test Data");
    let metadata = std::option::some(string::utf8(b"Test Metadata"));
    let state = notarization::new_state_from_string(data, metadata);

    // Create a dynamic notarization with no transfer lock
    dynamic_notarization::create(
        state,
        std::option::some(string::utf8(b"Test Description")),
        std::option::some(string::utf8(b"Test Updatable Metadata")),
        timelock::none(),
        &clock,
        ctx,
    );

    scenario.next_tx(ADMIN_ADDRESS);

    // Check that the notarization was created and transferred to ADMIN_ADDRESS
    let notarization = scenario.take_from_sender<notarization::Notarization<string::String>>();

    // Verify notarization properties
    assert!(notarization::notarization_method(&notarization).is_dynamic(), 0);
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

    // Check that it's transferable (no transfer lock)
    assert!(dynamic_notarization::is_transferable(&notarization, &clock), 0);

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    scenario.end();
}

#[test]
public fun test_create_dynamic_notarization_with_vector_data() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = scenario.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state with vector data
    let mut data = vector::empty<u8>();
    vector::push_back(&mut data, 1u8);
    vector::push_back(&mut data, 2u8);
    vector::push_back(&mut data, 3u8);
    let metadata = std::option::some(string::utf8(b"Test Metadata"));
    let state = notarization::new_state_from_bytes(data, metadata);

    // Create a dynamic notarization with no transfer lock
    dynamic_notarization::create(
        state,
        std::option::some(string::utf8(b"Test Description")),
        std::option::some(string::utf8(b"Test Updatable Metadata")),
        timelock::none(),
        &clock,
        ctx,
    );

    scenario.next_tx(ADMIN_ADDRESS);

    // Check that the notarization was created and transferred to ADMIN_ADDRESS
    let notarization = scenario.take_from_sender<notarization::Notarization<vector<u8>>>();

    // Verify notarization properties
    assert!(notarization::notarization_method(&notarization).is_dynamic(), 0);
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
    assert!(notarization::state(&notarization) == state, 0);

    // Check that it's transferable (no transfer lock)
    assert!(dynamic_notarization::is_transferable(&notarization, &clock), 0);

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    scenario.end();
}

#[test]
public fun test_create_dynamic_notarization_with_transfer_lock() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = scenario.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state with string data
    let data = string::utf8(b"Test Data");
    let metadata = std::option::some(string::utf8(b"Test Metadata"));
    let state = notarization::new_state_from_string(data, metadata);

    // Create a transfer lock that unlocks at timestamp 2000
    let transfer_lock = timelock::unlock_at(2000, &clock);

    // Create a dynamic notarization with transfer lock
    dynamic_notarization::create(
        state,
        std::option::some(string::utf8(b"Test Description")),
        std::option::some(string::utf8(b"Test Updatable Metadata")),
        transfer_lock,
        &clock,
        ctx,
    );

    scenario.next_tx(ADMIN_ADDRESS);

    // Check that the notarization was created and transferred to ADMIN_ADDRESS
    let notarization = scenario.take_from_sender<notarization::Notarization<string::String>>();

    // Verify it's not transferable (has transfer lock)
    assert!(!dynamic_notarization::is_transferable(&notarization, &clock), 0);

    // Advance time past the lock
    clock::increment_for_testing(&mut clock, 2000000);

    // Now it should be transferable
    assert!(dynamic_notarization::is_transferable(&notarization, &clock), 0);

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    scenario.end();
}

#[test]
public fun test_transfer_dynamic_notarization() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);

    // First transaction: create the notarization
    {
        let mut clock = clock::create_for_testing(scenario.ctx());
        clock::set_for_testing(&mut clock, 1000000);

        // Create a simple state
        let data = string::utf8(b"Test Data");
        let state = notarization::new_state_from_string(data, std::option::none());

        // Create a dynamic notarization with no transfer lock
        dynamic_notarization::create(
            state,
            std::option::none(),
            std::option::none(),
            timelock::none(),
            &clock,
            scenario.ctx(),
        );

        clock::destroy_for_testing(clock);
    };

    // Second transaction: transfer the notarization
    scenario.next_tx(ADMIN_ADDRESS);
    {
        let clock = clock::create_for_testing(scenario.ctx());

        // Take the notarization and transfer it to RECIPIENT_ADDRESS
        let notarization = scenario.take_from_sender<notarization::Notarization<string::String>>();
        dynamic_notarization::transfer(notarization, RECIPIENT_ADDRESS, &clock, scenario.ctx());

        clock::destroy_for_testing(clock);
    };

    // Third transaction: verify recipient received it
    scenario.next_tx(RECIPIENT_ADDRESS);
    {
        let ctx = scenario.ctx();
        let clock = clock::create_for_testing(ctx);

        // Verify that RECIPIENT_ADDRESS now has the notarization
        let notarization = scenario.take_from_sender<notarization::Notarization<string::String>>();

        // Clean up
        notarization::destroy(notarization, &clock);
        clock::destroy_for_testing(clock);
    };

    scenario.end();
}

#[test]
#[expected_failure(abort_code = dynamic_notarization::ECannotTransferLocked)]
public fun test_transfer_locked_dynamic_notarization() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);

    // First transaction: create the notarization
    {
        let mut clock = clock::create_for_testing(ctx(&mut scenario));
        clock::set_for_testing(&mut clock, 1000000);

        // Create a simple state
        let data = string::utf8(b"Test Data");
        let state = notarization::new_state_from_string(data, std::option::none());

        // Create a transfer lock that unlocks at timestamp 2000
        let transfer_lock = timelock::unlock_at(2000, &clock);

        // Create a dynamic notarization with transfer lock
        dynamic_notarization::create(
            state,
            std::option::none(),
            std::option::none(),
            transfer_lock,
            &clock,
            scenario.ctx(),
        );

        clock::destroy_for_testing(clock);
    };

    // Second transaction: try to transfer the locked notarization
    scenario.next_tx(ADMIN_ADDRESS);
    {
        let clock = clock::create_for_testing(scenario.ctx());

        // Take the notarization and try to transfer it while it's locked
        let notarization = scenario.take_from_sender<notarization::Notarization<string::String>>();

        // This should fail with ECannotTransferLocked
        dynamic_notarization::transfer(notarization, RECIPIENT_ADDRESS, &clock, scenario.ctx());

        // These should never be reached
        clock::destroy_for_testing(clock);
    };

    scenario.end();
}

#[test]
public fun test_update_dynamic_notarization() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = scenario.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state
    let data = string::utf8(b"Initial Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a dynamic notarization
    dynamic_notarization::create(
        state,
        std::option::none(),
        std::option::none(),
        timelock::none(),
        &clock,
        ctx,
    );

    scenario.next_tx(ADMIN_ADDRESS);

    // Take the notarization and update its state
    let mut notarization = scenario.take_from_sender<notarization::Notarization<string::String>>();

    // Create new state
    let new_data = string::utf8(b"Updated Data");
    let new_metadata = std::option::some(string::utf8(b"Updated Metadata"));
    let new_state = notarization::new_state_from_string(new_data, new_metadata);

    // Update the state
    notarization::update_state(&mut notarization, new_state, &clock);

    // Verify the update
    assert!(notarization::version_count(&notarization) == 1, 0);
    assert!(notarization::state(&notarization) == &new_state, 0);
    // Also update the updatable metadata
    notarization::update_metadata(
        &mut notarization,
        std::option::some(string::utf8(b"New Updatable Metadata")),
        &clock,
    );
    assert!(
        notarization::updatable_metadata(&notarization) == &std::option::some(string::utf8(b"New Updatable Metadata")),
        0,
    );

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    scenario.end();
}

#[test]
public fun test_dynamic_notarization_with_until_destroyed_lock() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = scenario.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state
    let data = string::utf8(b"Test Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a transfer lock that is locked until destroyed
    let transfer_lock = timelock::until_destroyed();

    // Create a dynamic notarization with transfer lock
    dynamic_notarization::create(
        state,
        std::option::none(),
        std::option::none(),
        transfer_lock,
        &clock,
        ctx,
    );

    scenario.next_tx(ADMIN_ADDRESS);

    // Check that the notarization was created and transferred to ADMIN_ADDRESS
    let notarization = scenario.take_from_sender<notarization::Notarization<string::String>>();

    // Verify it's not transferable (has until_destroyed transfer lock)
    assert!(!dynamic_notarization::is_transferable(&notarization, &clock), 0);

    // Advance time - should still be locked
    clock::increment_for_testing(&mut clock, 1000000);
    assert!(!dynamic_notarization::is_transferable(&notarization, &clock), 0);

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    scenario.end();
}

#[test]
public fun test_dynamic_notarization_with_none_lock() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = scenario.ctx();

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a simple state
    let data = string::utf8(b"Test Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a transfer lock that is never locked
    let transfer_lock = timelock::none();

    // Create a dynamic notarization with transfer lock
    dynamic_notarization::create(
        state,
        std::option::none(),
        std::option::none(),
        transfer_lock,
        &clock,
        ctx,
    );

    scenario.next_tx(ADMIN_ADDRESS);

    // Check that the notarization was created and transferred to ADMIN_ADDRESS
    let notarization = scenario.take_from_sender<notarization::Notarization<string::String>>();

    // Verify it's transferable (has none transfer lock)
    assert!(dynamic_notarization::is_transferable(&notarization, &clock), 0);
    assert!(notarization.lock_metadata().is_none(), 0);

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    scenario.end();
}
