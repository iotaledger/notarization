#[test_only]
module audit_trail::create_tests;

use audit_trail::main::{Self, AuditTrail, initial_admin_role_name};
use audit_trail::locking::{Self};
use audit_trail::capability::{Capability};
use iota::test_scenario::{Self as ts};
use iota::clock::{Self};
use std::string::{Self};

/// Test data type for audit trail records
public struct TestData has store, copy, drop {
    value: u64,
    message: vector<u8>,
}

fun destroy_capability(admin_cap: Capability, scenario: &ts::Scenario) {
    let mut trail = ts::take_shared<AuditTrail<TestData>>(scenario);
    trail.destroy_capability( admin_cap);
    ts::return_shared(trail);
}

#[test]
fun test_create_without_initial_record() {
    let user = @0xA;
    let mut scenario = ts::begin(user);
    
    {
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(1000);
        
        let locking_config = locking::new(std::option::none(), std::option::some(0));
        let trail_metadata = main::new_trail_metadata(
            std::option::some(string::utf8(b"Test Trail")),
            std::option::some(string::utf8(b"A test audit trail")),
        );
        
        let (admin_cap, trail_id) = main::create<TestData>(
            std::option::none(),
            std::option::none(),
            locking_config,
            trail_metadata,
            std::option::some(string::utf8(b"Updatable metadata")),
            &clock,
            ts::ctx(&mut scenario),
        );
        
        // Verify capability was created
        assert!(admin_cap.role() == initial_admin_role_name(), 0);
        assert!(admin_cap.trail_id() == trail_id, 1);
        
        // Clean up
        clock::destroy_for_testing(clock);
        destroy_capability(admin_cap, &scenario);
    };
    
    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        
        // Verify trail was created correctly
        assert!(trail.trail_creator() == user, 2);
        assert!(trail.trail_created_at() == 1000, 3);
        assert!(trail.trail_record_count() == 0, 4);
        
        ts::return_shared(trail);
    };
    
    ts::end(scenario);  
}

#[test]
fun test_create_with_initial_record() {
    let user = @0xB;
    let mut scenario = ts::begin(user);
    
    {
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(2000);
        
        let locking_config = locking::new(std::option::some(86400), std::option::none()); // 1 day in seconds
        let trail_metadata = main::new_trail_metadata(
            std::option::some(string::utf8(b"Test Trail with Record")),
            std::option::some(string::utf8(b"A test audit trail with initial record")),
        );
        
        let initial_data = TestData {
            value: 42,
            message: b"Hello, World!",
        };
        
        let (admin_cap, trail_id) = main::create<TestData>(
            std::option::some(initial_data),
            std::option::some(string::utf8(b"Initial record metadata")),
            locking_config,
            trail_metadata,
            std::option::some(string::utf8(b"Updatable metadata")),
            &clock,
            ts::ctx(&mut scenario),
        );
        
        // Verify capability
        assert!(admin_cap.role() == initial_admin_role_name(), 0);
        assert!(admin_cap.trail_id() == trail_id, 1);
        
        // Clean up
        clock::destroy_for_testing(clock);
        destroy_capability(admin_cap, &scenario);
    };
    
    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        
        // Verify trail with initial record
        assert!(trail.trail_creator() == user, 2);
        assert!(trail.trail_created_at() == 2000, 3);
        assert!(trail.trail_record_count() == 1, 4);
        
        // Verify the initial record exists
        assert!(trail.trail_has_record(0), 5);
        
        ts::return_shared(trail);
    };
    
    ts::end(scenario);
}

#[test]
fun test_create_minimal_metadata() {
    let user = @0xC;
    let mut scenario = ts::begin(user);
    
    {
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(3000);
        
        let locking_config = locking::new(std::option::none(), std::option::some(0));
        let trail_metadata = main::new_trail_metadata(
            std::option::none(),
            std::option::none(),
        );
        
        let (admin_cap, _trail_id) = main::create<TestData>(
            std::option::none(),
            std::option::none(),
            locking_config,
            trail_metadata,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );
        
        // Verify capability was created
        assert!(admin_cap.role() == initial_admin_role_name(), 0);
        
        // Clean up
        destroy_capability(admin_cap, &scenario);
        clock::destroy_for_testing(clock);
    };
    
    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        
        // Verify trail was created
        assert!(trail.trail_creator() == user, 1);
        assert!(trail.trail_created_at() == 3000, 2);
        assert!(trail.trail_record_count() == 0, 3);
        
        ts::return_shared(trail);
    };
    
    ts::end(scenario);
}

#[test]
fun test_create_with_locking_enabled() {
    let user = @0xD;
    let mut scenario = ts::begin(user);
    
    {
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(4000);
        
        let locking_config = locking::new(std::option::some(604800), std::option::none()); // 7 days in seconds
        let trail_metadata = main::new_trail_metadata(
            std::option::some(string::utf8(b"Locked Trail")),
            std::option::none(),
        );
        
        let (admin_cap, _trail_id) = main::create<TestData>(
            std::option::none(),
            std::option::none(),
            locking_config,
            trail_metadata,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );
        
        // Clean up
        destroy_capability(admin_cap, &scenario);
        clock::destroy_for_testing(clock);
    };
    
    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        
        // Verify trail with locking enabled
        assert!(trail.trail_creator() == user, 0);
        assert!(trail.trail_record_count() == 0, 1);
        
        ts::return_shared(trail);
    };
    
    ts::end(scenario);
}

#[test]
fun test_create_multiple_trails() {
    let user = @0xE;
    let mut scenario = ts::begin(user);
    
    let mut trail_ids = vector::empty<iota::object::ID>();
    
    // Create first trail
    {
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(5000);
        
        let locking_config = locking::new(std::option::none(), std::option::some(0));
        let trail_metadata = main::new_trail_metadata(
            std::option::some(string::utf8(b"Trail 1")),
            std::option::none(),
        );
        
        let (admin_cap1, trail_id1) = main::create<TestData>(
            std::option::none(),
            std::option::none(),
            locking_config,
            trail_metadata,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );
        
        trail_ids.push_back(trail_id1);
        ts::return_to_sender(&scenario, admin_cap1);
        clock::destroy_for_testing(clock);
    };
    
    ts::next_tx(&mut scenario, user);
    
    // Create second trail
    {
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(6000);
        
        let locking_config = locking::new(std::option::none(), std::option::some(0));
        let trail_metadata = main::new_trail_metadata(
            std::option::some(string::utf8(b"Trail 2")),
            std::option::none(),
        );
        
        let (admin_cap2, trail_id2) = main::create<TestData>(
            std::option::none(),
            std::option::none(),
            locking_config,
            trail_metadata,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );
        
        trail_ids.push_back(trail_id2);
        
        // Verify trails have different IDs
        assert!(trail_ids[0] != trail_ids[1], 0);
        
        ts::return_to_sender(&scenario, admin_cap2);
        clock::destroy_for_testing(clock);
    };
    
    ts::end(scenario);
}
