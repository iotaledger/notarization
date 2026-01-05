#[test_only]
/// This module contains comprehensive tests for the AuditTrail creation functionality.
module audit_trail::create_audit_trail_tests;

use audit_trail::main::{Self, AuditTrail, initial_admin_role_name};
use audit_trail::locking::{Self};
use audit_trail::test_utils::{setup_test_audit_trail, new_test_data, initial_time_for_testing, TestData, fetch_capability_trail_and_clock, cleanup_capability_trail_and_clock};
use iota::test_scenario::{Self as ts};
use iota::clock::{Self};
use std::string::{Self};

/// Goals of this test:
/// - Verifies creating an AuditTrail with no initial record
/// - Checks admin capability creation with correct role and trail_id
/// - Validates trail metadata (creator, creation time, record count)
#[test]
fun test_create_without_initial_record() {
    let user = @0xA;
    let mut scenario = ts::begin(user);
    
    {
        let locking_config = locking::new(locking::window_count_based(0));
        
        let (admin_cap, trail_id) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none()
        );
        
        // Verify capability was created
        assert!(admin_cap.role() == initial_admin_role_name(), 0);
        assert!(admin_cap.security_vault_id() == trail_id, 1);
        
        // Clean up
        admin_cap.destroy_for_testing();
    };
    
    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        
        // Verify trail was created correctly
        assert!(trail.trail_creator() == user, 2);
        assert!(trail.trail_created_at() == initial_time_for_testing(), 3);
        assert!(trail.trail_record_count() == 0, 4);
        
        ts::return_shared(trail);
    };
    
    ts::end(scenario);  
}

/// Goals of this test:
/// - Tests AuditTrail creation with an initial record
/// - Verifies the trail contains exactly one record after creation
/// - Validates the initial record exists at index 0
#[test]
fun test_create_with_initial_record() {
    let user = @0xB;
    let mut scenario = ts::begin(user);
    
    {
        let locking_config = locking::new(locking::window_time_based(86400)); // 1 day in seconds
        let initial_data = new_test_data(42, b"Hello, World!");
        
        let (admin_cap, trail_id) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(initial_data)
        );
        
        // Verify capability
        assert!(admin_cap.role() == initial_admin_role_name(), 0);
        assert!(admin_cap.security_vault_id() == trail_id, 1);
        
        // Clean up
        admin_cap.destroy_for_testing();
    };
    
    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        
        // Verify trail with initial record
        assert!(trail.trail_creator() == user, 2);
        assert!(trail.trail_created_at() == initial_time_for_testing(), 3);
        assert!(trail.trail_record_count() == 1, 4);
        
        // Verify the initial record exists
        assert!(trail.trail_has_record(0), 5);
        
        ts::return_shared(trail);
    };
    
    ts::end(scenario);
}

/// Goals of this test:
/// - Tests creating a trail with minimal metadata (optional fields set to none)
/// - Uses a custom clock time to verify timestamp handling
/// - Ensures the system handles minimal configuration correctly
#[test]
fun test_create_minimal_metadata() {
    let user = @0xC;
    let mut scenario = ts::begin(user);
    
    {
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(3000);
        
        let locking_config = locking::new(locking::window_count_based(0));
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
        admin_cap.destroy_for_testing();
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

/// Goals of this test:
/// - Verifies AuditTrail creation with locking configuration enabled
/// - Tests a 7-day time-based lock period
/// - Validates the trail is created successfully with locking constraints
#[test]
fun test_create_with_locking_enabled() {
    let user = @0xD;
    let mut scenario = ts::begin(user);
    
    {       
        let locking_config = locking::new(locking::window_time_based(604800)); // 7 days in seconds
        let (admin_cap, _trail_id) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none()
        );
        
        // Clean up
        admin_cap.destroy_for_testing();
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

/// Goals of this test:
/// - Tests creating multiple independent AuditTrail instances
/// - Verifies each trail receives a unique ID
/// - Ensures multiple trails can coexist without conflicts
#[test]
fun test_create_multiple_trails() {
    let user = @0xE;
    let mut scenario = ts::begin(user);
    
    let mut trail_ids = vector::empty<iota::object::ID>();
    
    // Create first trail
    {        
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap1, trail_id1) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none()
        );
        
        trail_ids.push_back(trail_id1);
        admin_cap1.destroy_for_testing();
    };
    
    ts::next_tx(&mut scenario, user);
    
    // Create second trail
    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap2, trail_id2) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none()
        );
        
        trail_ids.push_back(trail_id2);
        
        // Verify trails have different IDs
        assert!(trail_ids[0] != trail_ids[1], 0);
        
        admin_cap2.destroy_for_testing();
    };
    
    ts::end(scenario);
}

/// Test creating a MetadataAdmin role with metadata_admin_permissions.
/// 
/// This test verifies that:
/// 1. A creator can create an AuditTrail and receive an admin capability
/// 2. The admin capability can be transferred to another user
/// 3. The user can use the capability to create a new MetadataAdmin role
/// 4. The new role has the correct permissions (meta_data_update and meta_data_delete)
#[test]
fun test_create_metadata_admin_role() {
    let creator = @0xA;
    let user = @0xB;
    let mut scenario = ts::begin(creator);
    
    // Creator creates the audit trail
    {        
        let locking_config = locking::new(locking::window_count_based(0));
        
        let (admin_cap, trail_id) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none()
        );
        
        // Verify admin capability was created
        assert!(admin_cap.role() == initial_admin_role_name(), 0);
        assert!(admin_cap.security_vault_id() == trail_id, 1);
        
        // Transfer the admin capability to the user
        transfer::public_transfer(admin_cap, user);        
    };
    
    // User receives the capability and creates the MetadataAdmin role
    ts::next_tx(&mut scenario, user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);        
        // Create the MetadataAdmin role using the admin capability
        let metadata_admin_role_name = string::utf8(b"MetadataAdmin");
        let metadata_admin_perms = audit_trail::permission::metadata_admin_permissions();
        
        trail.roles_mut().create_role(
            &admin_cap,
            metadata_admin_role_name,
            metadata_admin_perms,
            &clock,
            ts::ctx(&mut scenario),
        );
        
        // Verify the role was created by fetching its permissions
        let role_perms = trail.roles().get_role_permissions(&string::utf8(b"MetadataAdmin"));
        
        // Verify the role has the correct permissions
        assert!(audit_trail::permission::has_permission(role_perms, &audit_trail::permission::update_metadata()), 2);
        assert!(audit_trail::permission::has_permission(role_perms, &audit_trail::permission::delete_metadata()), 3);
        assert!(iota::vec_set::size(role_perms) == 2, 4);
        
        // Clean up
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock); 
    };
    
    ts::end(scenario);
}
