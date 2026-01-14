#[test_only]
module audit_trail::capability_tests;

use audit_trail::{
    capability::Capability,
    locking,
    main::AuditTrail,
    permission,
    test_utils::{Self, TestData, setup_test_audit_trail}
};
use iota::test_scenario as ts;
use std::string;

/// Test that new_capability() correctly creates a capability and tracks it in issued_capabilities.
///
/// This test validates:
/// - Capability is created with correct role and trail ID
/// - Capability ID is added to the audit trail's issued_capabilities set
/// - Multiple capabilities can be issued and all are tracked
/// - Each capability has a unique ID
#[test]
fun test_new_capability() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

    let mut scenario = ts::begin(admin_user);

    // Setup: Create audit trail with admin capability
    let trail_id = {
        let locking_config = locking::new(locking::window_count_based(0));

        let (admin_cap, trail_id) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            option::none(),
        );

        transfer::public_transfer(admin_cap, admin_user);
        trail_id
    };

    // Create a custom role for testing
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let record_admin_perms = permission::record_admin_permissions();
        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            record_admin_perms,
            ts::ctx(&mut scenario),
        );

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Test: Issue first capability
    ts::next_tx(&mut scenario, admin_user);
    let cap1_id = {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Verify initial state - only admin capability should be tracked
        let initial_cap_count = trail.issued_capabilities().size();
        assert!(initial_cap_count == 1, 0); // Only admin cap

        let cap1 = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );

        // Verify capability was created correctly
        assert!(cap1.role() == string::utf8(b"RecordAdmin"), 1);
        assert!(cap1.trail_id() == trail_id, 2);

        let cap1_id = object::id(&cap1);

        // Verify capability ID is tracked in issued_capabilities
        assert!(trail.issued_capabilities().size() == initial_cap_count + 1, 3);
        assert!(trail.issued_capabilities().contains(&cap1_id), 4);

        transfer::public_transfer(cap1, user1);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);

        cap1_id
    };

    // Test: Issue second capability
    ts::next_tx(&mut scenario, admin_user);
    let _cap2_id = {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let previous_cap_count = trail.issued_capabilities().size();

        let cap2 = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );

        let cap2_id = object::id(&cap2);

        // Verify both capabilities are tracked
        assert!(trail.issued_capabilities().size() == previous_cap_count + 1, 5);
        assert!(trail.issued_capabilities().contains(&cap1_id), 6);
        assert!(trail.issued_capabilities().contains(&cap2_id), 7);

        // Verify capabilities have unique IDs
        assert!(cap1_id != cap2_id, 8);

        transfer::public_transfer(cap2, user2);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);

        cap2_id
    };

    ts::end(scenario);
}

/// Test that revoke_capability() correctly revokes a capability and removes it from issued_capabilities.
///
/// This test validates:
/// - Capability can be revoked by an authorized user
/// - Revoked capability ID is removed from issued_capabilities set
/// - Revoking one capability doesn't affect other capabilities
/// - Revoked capability object is properly destroyed
#[test]
fun test_revoke_capability() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

    let mut scenario = ts::begin(admin_user);

    // Setup: Create audit trail with admin capability
    let _trail_id = {
        let locking_config = locking::new(locking::window_count_based(0));

        let (admin_cap, trail_id) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            option::none(),
        );

        transfer::public_transfer(admin_cap, admin_user);
        trail_id
    };

    // Create a custom role for testing
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let record_admin_perms = permission::record_admin_permissions();
        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            record_admin_perms,
            ts::ctx(&mut scenario),
        );

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Issue two capabilities
    ts::next_tx(&mut scenario, admin_user);
    let (cap1_id, cap2_id) = {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let cap1 = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );
        let cap1_id = object::id(&cap1);
        transfer::public_transfer(cap1, user1);

        let cap2 = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );
        let cap2_id = object::id(&cap2);
        transfer::public_transfer(cap2, user2);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);

        (cap1_id, cap2_id)
    };

    // Test: Revoke first capability
    ts::next_tx(&mut scenario, user1);
    {
        let admin_cap = ts::take_from_address<Capability>(&scenario, admin_user);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let cap1 = ts::take_from_sender<Capability>(&scenario);

        // Verify both capabilities are tracked before revocation
        let cap_count_before = trail.issued_capabilities().size();
        assert!(trail.issued_capabilities().contains(&cap1_id), 0);
        assert!(trail.issued_capabilities().contains(&cap2_id), 1);

        // Revoke the capability
        trail.revoke_capability(
            &admin_cap,
            cap1.id(),
        );

        // Verify capability was removed from tracking
        assert!(trail.issued_capabilities().size() == cap_count_before - 1, 2);
        assert!(!trail.issued_capabilities().contains(&cap1_id), 3);

        // Verify other capability is still tracked
        assert!(trail.issued_capabilities().contains(&cap2_id), 4);

        ts::return_to_address(admin_user, admin_cap);
        ts::return_to_sender(&scenario, cap1);
        ts::return_shared(trail);
    };

    // Verify cap1 is still available to user1 -it has been revoked, not destroyed
    ts::next_tx(&mut scenario, user1);
    {
        // This should not find cap1 since it was revoked
        assert!(ts::has_most_recent_for_sender<Capability>(&scenario), 5);
    };

    // Test: Revoke second capability
    ts::next_tx(&mut scenario, user2);
    {
        let admin_cap = ts::take_from_address<Capability>(&scenario, admin_user);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let cap2 = ts::take_from_sender<Capability>(&scenario);

        let cap_count_before = trail.issued_capabilities().size();

        trail.revoke_capability(
            &admin_cap,
            cap2.id(),
        );

        // Verify capability was removed from tracking
        assert!(trail.issued_capabilities().size() == cap_count_before - 1, 6);
        assert!(!trail.issued_capabilities().contains(&cap2_id), 7);

        ts::return_to_address(admin_user, admin_cap);
        ts::return_to_sender(&scenario, cap2);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

/// Test that destroy_capability() correctly destroys a capability and removes it from issued_capabilities.
///
/// This test validates:
/// - Capability owner can destroy their own capability
/// - Destroyed capability ID is removed from issued_capabilities set
/// - Destroying one capability doesn't affect other capabilities
/// - Capability object is properly destroyed and cannot be used again
#[test]
fun test_destroy_capability() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

    let mut scenario = ts::begin(admin_user);

    // Setup: Create audit trail with admin capability
    let trail_id = {
        let locking_config = locking::new(locking::window_count_based(0));

        let (admin_cap, trail_id) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            option::none(),
        );

        transfer::public_transfer(admin_cap, admin_user);
        trail_id
    };

    // Create a custom role for testing
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let record_admin_perms = permission::record_admin_permissions();
        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            record_admin_perms,
            ts::ctx(&mut scenario),
        );

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Issue two capabilities
    ts::next_tx(&mut scenario, admin_user);
    let (cap1_id, cap2_id) = {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let cap1 = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );
        let cap1_id = object::id(&cap1);
        transfer::public_transfer(cap1, user1);

        let cap2 = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );
        let cap2_id = object::id(&cap2);
        transfer::public_transfer(cap2, user2);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);

        (cap1_id, cap2_id)
    };

    // Test: User1 destroys their own capability
    ts::next_tx(&mut scenario, user1);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let cap1 = ts::take_from_sender<Capability>(&scenario);

        // Verify both capabilities are tracked before destruction
        let cap_count_before = trail.issued_capabilities().size();
        assert!(trail.issued_capabilities().contains(&cap1_id), 0);
        assert!(trail.issued_capabilities().contains(&cap2_id), 1);

        // Destroy the capability
        trail.destroy_capability(cap1);

        // Verify capability was removed from tracking
        assert!(trail.issued_capabilities().size() == cap_count_before - 1, 2);
        assert!(!trail.issued_capabilities().contains(&cap1_id), 3);

        // Verify other capability is still tracked
        assert!(trail.issued_capabilities().contains(&cap2_id), 4);

        ts::return_shared(trail);
    };

    // Verify cap1 is no longer available to user1
    ts::next_tx(&mut scenario, user1);
    {
        // This should not find cap1 since it was destroyed
        assert!(!ts::has_most_recent_for_sender<Capability>(&scenario), 5);
    };

    // Test: User2 destroys their own capability
    ts::next_tx(&mut scenario, user2);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let cap2 = ts::take_from_sender<Capability>(&scenario);

        let cap_count_before = trail.issued_capabilities().size();

        trail.destroy_capability(cap2);

        // Verify capability was removed from tracking
        assert!(trail.issued_capabilities().size() == cap_count_before - 1, 6);
        assert!(!trail.issued_capabilities().contains(&cap2_id), 7);

        ts::return_shared(trail);
    };

    // Verify only admin capability remains
    ts::next_tx(&mut scenario, admin_user);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Only the initial admin capability should remain
        assert!(trail.issued_capabilities().size() == 1, 8);

        ts::return_shared(trail);
    };

    ts::end(scenario);
}

/// Test capability lifecycle: creation, usage, and destruction in a complete workflow.
///
/// This test validates:
/// - Multiple capabilities can be created for different roles
/// - Capabilities can be used to perform authorized actions
/// - Capabilities can be revoked or destroyed
/// - issued_capabilities tracking remains accurate throughout the lifecycle
#[test]
fun test_capability_lifecycle() {
    let admin_user = @0xAD;
    let record_admin_user = @0xB0B;
    let role_admin_user = @0xCAB;

    let mut scenario = ts::begin(admin_user);

    // Setup: Create audit trail
    let trail_id = {
        let locking_config = locking::new(locking::window_count_based(0));

        let (admin_cap, trail_id) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            option::none(),
        );

        transfer::public_transfer(admin_cap, admin_user);
        trail_id
    };

    // Create roles
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Initially only admin cap should be tracked
        assert!(trail.issued_capabilities().size() == 1, 0);

        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            permission::record_admin_permissions(),
            ts::ctx(&mut scenario),
        );

        trail.create_role(
            &admin_cap,
            string::utf8(b"RoleAdmin"),
            permission::role_admin_permissions(),
            ts::ctx(&mut scenario),
        );

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Issue capabilities
    ts::next_tx(&mut scenario, admin_user);
    let (record_cap_id, role_cap_id) = {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let record_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );
        let record_cap_id = object::id(&record_cap);
        transfer::public_transfer(record_cap, record_admin_user);

        let role_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RoleAdmin"),
            ts::ctx(&mut scenario),
        );
        let role_cap_id = object::id(&role_cap);
        transfer::public_transfer(role_cap, role_admin_user);

        // Verify all capabilities are tracked
        assert!(trail.issued_capabilities().size() == 3, 1); // admin + record + role
        assert!(trail.issued_capabilities().contains(&record_cap_id), 2);
        assert!(trail.issued_capabilities().contains(&role_cap_id), 3);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);

        (record_cap_id, role_cap_id)
    };

    // Use RecordAdmin capability to add a record
    ts::next_tx(&mut scenario, record_admin_user);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        let mut clock = iota::clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(test_utils::initial_time_for_testing() + 1000);

        let test_data = test_utils::new_test_data(1, b"Test record");
        trail.add_record(
            &record_cap,
            test_data,
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        iota::clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, record_cap);
        ts::return_shared(trail);
    };

    // RecordAdmin destroys their capability
    ts::next_tx(&mut scenario, record_admin_user);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        trail.destroy_capability(record_cap);

        // Verify capability was removed
        assert!(trail.issued_capabilities().size() == 2, 4); // admin + role
        assert!(!trail.issued_capabilities().contains(&record_cap_id), 5);

        ts::return_shared(trail);
    };

    // Admin revokes RoleAdmin capability
    ts::next_tx(&mut scenario, role_admin_user);
    {
        let admin_cap = ts::take_from_address<Capability>(&scenario, admin_user);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let role_cap = ts::take_from_sender<Capability>(&scenario);

        trail.revoke_capability(
            &admin_cap,
            role_cap.id(),
        );

        // Verify capability was removed
        assert!(trail.issued_capabilities().size() == 1, 6); // only admin remains
        assert!(!trail.issued_capabilities().contains(&role_cap_id), 7);

        ts::return_to_address(admin_user, admin_cap);
        ts::return_to_sender(&scenario, role_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}
