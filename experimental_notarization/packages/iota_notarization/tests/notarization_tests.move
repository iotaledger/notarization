// Copyright (c) 2024 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides tests for the core notarization module
#[test_only]
module iota_notarization::notarization_tests;

use std::string;
use iota::clock;
use iota::test_scenario::{Self as ts};
use iota_notarization::timelock;
use iota_notarization::notarization;

const ADMIN_ADDRESS: address = @0x1;

public struct ComplexObject has store, drop, copy {
    field1: u64,
    field2: string::String,
    field3: vector<u8>,
}

fun create_complex_object_with_data(
    field1: u64,
    field2: string::String,
    field3: vector<u8>,
): ComplexObject {
    ComplexObject {
        field1: field1,
        field2: field2,
        field3: field3,
    }
}

#[test]
public fun test_create_notarization_with_complex_object() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);


    let mut field3 = vector::empty();
    field3.push_back(1u8);
    field3.push_back(2u8);
    let complex = create_complex_object_with_data(1, string::utf8(b"test"), field3);

    let state = notarization::new_state_from_generic(complex, std::option::none());

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    let notarization = notarization::new_dynamic_notarization(state, std::option::some(string::utf8(b"Test Description")), std::option::some(string::utf8(b"Test Updateable Metadata")), std::option::none(), &clock, ctx);
    scenario.next_tx(ADMIN_ADDRESS);

    notarization::transfer_notarization(notarization, ADMIN_ADDRESS);

    scenario.next_tx(ADMIN_ADDRESS);

    // Check that the notarization was created and transferred to ADMIN_ADDRESS
    let notarization = scenario.take_from_sender<notarization::Notarization<ComplexObject>>();

    // Verify notarization properties
    assert!(notarization::notarization_method(&notarization).is_dynamic(), 0);
    assert!(notarization::description(&notarization) == &std::option::some(string::utf8(b"Test Description")), 0);
    assert!(notarization::updateable_metadata(&notarization) == &std::option::some(string::utf8(b"Test Updateable Metadata")), 0);
    assert!(notarization::last_change(&notarization) == 1000000, 0);
    assert!(notarization::version_count(&notarization) == 0, 0);
    assert!(notarization::state(&notarization) == state, 0);

    // Check the state
    assert!(notarization::state(&notarization) == state, 0);

    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = notarization::EUntilDestroyedLockNotAllowed)]
public fun test_until_destroyed_delete_lock_not_allowed() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let clock = clock::create_for_testing(ctx);

    // Try to create lock metadata with until_destroyed delete lock - should fail
    let update_lock = timelock::none();
    let delete_lock = timelock::until_destroyed();
    let transfer_lock = timelock::none();

    let lock_metadata = notarization::new_lock_metadata(update_lock, delete_lock, transfer_lock);

    // Clean up - won't reach here due to expected failure
    notarization::destroy_lock_metadata(lock_metadata, &clock);
    clock::destroy_for_testing(clock);
    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = notarization::ELockTimeNotSatisfied)]
public fun test_invalid_lock_time_ordering() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let clock = clock::create_for_testing(ctx);

    // Try to create lock metadata with invalid time ordering
    // Delete lock time must be >= update lock time
    let update_lock = timelock::new_unlock_at(2000000, &clock);
    let delete_lock = timelock::new_unlock_at(1500000, &clock); // Earlier than update_lock
    let transfer_lock = timelock::none();

    let lock_metadata = notarization::new_lock_metadata(update_lock, delete_lock, transfer_lock);

    // Clean up - won't reach here due to expected failure
    notarization::destroy_lock_metadata(lock_metadata, &clock);
    clock::destroy_for_testing(clock);
    ts::end(scenario);
}

#[test]
public fun test_state_updates_and_versioning() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create initial state
    let data = string::utf8(b"Initial Data");
    let metadata = std::option::some(string::utf8(b"Initial Metadata"));
    let state = notarization::new_state_from_string(data, metadata);

    // Create a dynamic notarization (no locks)
    let mut notarization = notarization::new_dynamic_notarization(
        state,
        std::option::some(string::utf8(b"Test Description")),
        std::option::some(string::utf8(b"Test Updateable Metadata")),
        std::option::none(),
        &clock,
        ctx
    );

    // Verify initial state
    assert!(notarization::version_count(&notarization) == 0, 0);

    // Update state
    let new_data = string::utf8(b"Updated Data");
    let new_metadata = std::option::some(string::utf8(b"Updated Metadata"));
    let new_state = notarization::new_state_from_string(new_data, new_metadata);

    clock::increment_for_testing(&mut clock, 1000);
    notarization::update_state(&mut notarization, new_state, &clock);

    // Verify state was updated
    assert!(notarization::version_count(&notarization) == 1, 0);

    // Update metadata
    clock::increment_for_testing(&mut clock, 1000);
    notarization::update_metadata(&mut notarization, std::option::some(string::utf8(b"New Metadata")), &clock);

    // Verify metadata was updated
    assert!(notarization::updateable_metadata(&notarization) == &std::option::some(string::utf8(b"New Metadata")), 0);

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);
    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = notarization::EUpdateWhileLocked)]
public fun test_update_while_locked() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create initial state
    let data = string::utf8(b"Initial Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a locked notarization with update lock
    let mut notarization = notarization::new_locked_notarization(
        state,
        std::option::none(),
        std::option::none(),
        timelock::none(), // delete lock
        &clock,
        ctx
    );

    // Try to update while locked (locked notarizations are always locked for updates)
    let new_data = string::utf8(b"Updated Data");
    let new_state = notarization::new_state_from_string(new_data, std::option::none());
    notarization::update_state(&mut notarization, new_state, &clock);

    // Clean up - won't reach here due to expected failure
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);
    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = notarization::EDestroyWhileLocked)]
public fun test_destroy_while_locked() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create initial state
    let data = string::utf8(b"Initial Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a locked notarization with delete lock
    let delete_lock = timelock::new_unlock_at(2000, &clock);
    let notarization = notarization::new_locked_notarization(
        state,
        std::option::none(),
        std::option::none(),
        delete_lock,
        &clock,
        ctx
    );

    // Try to destroy while locked
    notarization::destroy(notarization, &clock);

    // Clean up - won't reach here due to expected failure
    clock::destroy_for_testing(clock);
    ts::end(scenario);
}

#[test]
public fun test_lock_status_checks() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create initial state
    let data = string::utf8(b"Test Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create a locked notarization with timed locks
    let delete_lock = timelock::new_unlock_at(2000, &clock);
    let notarization = notarization::new_locked_notarization(
        state,
        std::option::none(),
        std::option::none(),
        delete_lock,
        &clock,
        ctx
    );

    // Check initial lock status
    assert!(notarization::is_update_locked(&notarization, &clock), 0); // Always locked for updates
    assert!(notarization::is_delete_locked(&notarization, &clock), 0); // Locked until 2000000
    assert!(notarization::is_transfer_locked(&notarization, &clock), 0); // Always locked for transfers

    // Advance time past delete lock
    clock::increment_for_testing(&mut clock, 1000001);

    // Check lock status after time advance
    assert!(notarization::is_update_locked(&notarization, &clock), 0); // Still locked for updates
    assert!(!notarization::is_delete_locked(&notarization, &clock), 0); // Now unlocked for deletion
    assert!(notarization::is_transfer_locked(&notarization, &clock), 0); // Still locked for transfers

    // Clean up
    notarization::destroy(notarization, &clock);
    clock::destroy_for_testing(clock);
    ts::end(scenario);
}

#[test]
public fun test_method_type_checks() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create a dynamic notarization
    let data = string::utf8(b"Test Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    let dynamic_notarization = notarization::new_dynamic_notarization(
        state,
        std::option::none(),
        std::option::none(),
        std::option::none(),
        &clock,
        ctx
    );

    // Verify dynamic type
    assert!(notarization::notarization_method(&dynamic_notarization).is_dynamic(), 0);
    assert!(!notarization::notarization_method(&dynamic_notarization).is_locked(), 0);

    // Create a locked notarization
    let locked_notarization = notarization::new_locked_notarization(
        state,
        std::option::none(),
        std::option::none(),
        timelock::none(),
        &clock,
        ctx
    );

    // Verify locked type
    assert!(!notarization::notarization_method(&locked_notarization).is_dynamic(), 0);
    assert!(notarization::notarization_method(&locked_notarization).is_locked(), 0);

    // Clean up
    notarization::destroy(dynamic_notarization, &clock);
    notarization::destroy(locked_notarization, &clock);
    clock::destroy_for_testing(clock);
    ts::end(scenario);
}

#[test]
public fun test_notarization_with_all_locks() {
    let mut scenario = ts::begin(ADMIN_ADDRESS);
    let ctx = ts::ctx(&mut scenario);

    let mut clock = clock::create_for_testing(ctx);
    clock::set_for_testing(&mut clock, 1000000);

    // Create initial state
    let data = string::utf8(b"Test Data");
    let state = notarization::new_state_from_string(data, std::option::none());

    // Create locks with different expiration times
    let update_lock = timelock::new_unlock_at(1500, &clock); // Unlocks first
    let transfer_lock = timelock::new_unlock_at(2000, &clock); // Unlocks second
    let delete_lock = timelock::new_unlock_at(2500, &clock); // Unlocks last

    let lock_metadata = notarization::new_lock_metadata(update_lock, delete_lock, transfer_lock);

    // Create a dynamic notarization with all locks
    let mut notarization = notarization::create_custom_notarization(
        state,
        std::option::some(string::utf8(b"Test Description")),
        std::option::some(string::utf8(b"Test Metadata")),
        std::option::some(lock_metadata),
        &clock,
        ctx
    );

    // Initial state - all locks should be active
    assert!(notarization::is_update_locked(&notarization, &clock), 0);
    assert!(notarization::is_delete_locked(&notarization, &clock), 0);
    assert!(notarization::is_transfer_locked(&notarization, &clock), 0);

    // Advance time to just before update unlock (1499999)
    clock::increment_for_testing(&mut clock, 499999);
    assert!(notarization::is_update_locked(&notarization, &clock), 0);

    // Advance time to just after update unlock (1500001)
    clock::increment_for_testing(&mut clock, 2);
    assert!(!notarization::is_update_locked(&notarization, &clock), 0);

    // Try updating after unlock
    let new_data = string::utf8(b"Updated Data");
    let new_state = notarization::new_state_from_string(new_data, std::option::none());
    notarization::update_state(&mut notarization, new_state, &clock);

    // Verify update worked
    assert!(notarization::version_count(&notarization) == 1, 0);

    // Advance time to just before transfer unlock (1999999)
    clock::increment_for_testing(&mut clock, 499998);
    assert!(notarization::is_transfer_locked(&notarization, &clock), 0);

    // Advance time to just after transfer unlock (2000001)
    clock::increment_for_testing(&mut clock, 2);
    assert!(!notarization::is_transfer_locked(&notarization, &clock), 0);



    // Now we can transfer the notarization
    notarization::transfer_notarization(notarization, ADMIN_ADDRESS);

    scenario.next_tx(ADMIN_ADDRESS);

    // Take it back and verify it's still the same notarization
    let notarization = scenario.take_from_sender<notarization::Notarization<string::String>>();
    assert!(notarization::version_count(&notarization) == 1, 0);
    assert!(notarization::description(&notarization) == &std::option::some(string::utf8(b"Test Description")), 0);

    // Advance time to just before delete unlock (2499999)
    clock::increment_for_testing(&mut clock, 499998);
    assert!(notarization::is_delete_locked(&notarization, &clock), 0);

    // Advance time to just after delete unlock (2500000)
    clock::increment_for_testing(&mut clock, 2);
    assert!(!notarization::is_delete_locked(&notarization, &clock), 0);

    // Finally, we can destroy it since the delete lock is expired
    notarization::destroy(notarization, &clock);

    clock::destroy_for_testing(clock);
    ts::end(scenario);
}