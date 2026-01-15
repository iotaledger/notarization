#[test_only]
module audit_trail::capability_tests;

use audit_trail::{
    capability::Capability,
    locking,
    main::{Self, AuditTrail},
    permission,
    test_utils::{
        Self,
        TestData,
        setup_test_audit_trail,
        fetch_capability_trail_and_clock,
        cleanup_capability_trail_and_clock
    }
};
use iota::test_scenario::{Self as ts, Scenario};
use std::string;

/// Helper function to setup an audit trail with a RecordAdmin role and a capability
/// with a time window restriction transferred to the record_user.
/// Returns the trail_id.
fun setup_trail_with_record_admin_capability_and_time_window_restriction(
    scenario: &mut Scenario,
    admin_user: address,
    record_user: address,
    valid_from_secs: u64,
    valid_until_secs: u64,
): ID {
    // Setup
    let trail_id = setup_trail_with_record_admin_role(scenario, admin_user);

    // Issue capability with time window
    ts::next_tx(scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(scenario);

        let cap = trail
            .roles_mut()
            .new_capability(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                std::option::none(), // no address restriction
                std::option::some(valid_from_secs),
                std::option::some(valid_until_secs),
                &clock,
                ts::ctx(scenario),
            );

        // Verify capability properties
        assert!(cap.issued_to().is_none(), 0);
        assert!(cap.valid_from() == std::option::some(valid_from_secs), 1);
        assert!(cap.valid_until() == std::option::some(valid_until_secs), 2);

        transfer::public_transfer(cap, record_user);
        cleanup_capability_trail_and_clock(scenario, admin_cap, trail, clock);
    };

    trail_id
}

/// Helper function to setup an audit trail with a RecordAdmin role.
/// Returns the trail_id.
fun setup_trail_with_record_admin_role(scenario: &mut Scenario, admin_user: address): ID {
    // Setup: Create audit trail with admin capability
    let trail_id = {
        let locking_config = locking::new(locking::window_count_based(0));

        let (admin_cap, trail_id) = setup_test_audit_trail(
            scenario,
            locking_config,
            std::option::none(),
        );

        transfer::public_transfer(admin_cap, admin_user);
        trail_id
    };

    // Create a custom role for testing
    ts::next_tx(scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(scenario);
        let clock = iota::clock::create_for_testing(ts::ctx(scenario));

        let record_admin_perms = permission::record_admin_permissions();
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                record_admin_perms,
                &clock,
                ts::ctx(scenario),
            );

        iota::clock::destroy_for_testing(clock);
        ts::return_to_sender(scenario, admin_cap);
        ts::return_shared(trail);
    };

    trail_id
}

#[test]
fun test_new_capability() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

        transfer::public_transfer(cap, record_user);
        cleanup_capability_trail_and_clock(scenario, admin_cap, trail, clock);
    };

    let trail_id = {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, trail_id) = setup_test_audit_trail(
            scenario,
            locking_config,
            option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
        trail_id
    };

    // Create a role to issue capabilities for
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(scenario);
        let clock = iota::clock::create_for_testing(ts::ctx(scenario));

        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            permission::record_admin_permissions(),
            ts::ctx(&mut scenario),
        );

        iota::clock::destroy_for_testing(clock);
        ts::return_to_sender(scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Issue first capability and verify it's tracked
    ts::next_tx(&mut scenario, admin_user);
    let cap1_id = {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let initial_cap_count = trail.issued_capabilities().size();
        // Verify initial state - only admin capability should be tracked
        let initial_cap_count = trail.roles().issued_capabilities().size();
        assert!(initial_cap_count == 1, 0); // Only admin cap

        let cap1 = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );

        assert!(cap1.role() == string::utf8(b"RecordAdmin"), 1);
        assert!(cap1.security_vault_id() == trail_id, 2);

        let cap1_id = object::id(&cap1);

        // Verify capability ID is tracked in issued_capabilities
        assert!(trail.roles().issued_capabilities().size() == initial_cap_count + 1, 3);
        assert!(trail.roles().issued_capabilities().contains(&cap1_id), 4);

        transfer::public_transfer(cap1, user1);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);

        cap1_id
    };

    // Issue second capability and verify both are tracked with unique IDs
    ts::next_tx(&mut scenario, admin_user);
    let _cap2_id = {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let previous_cap_count = trail.roles().issued_capabilities().size();

        let cap2 = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );

        let cap2_id = object::id(&cap2);

        // Verify both capabilities are tracked
        assert!(trail.roles().issued_capabilities().size() == previous_cap_count + 1, 5);
        assert!(trail.roles().issued_capabilities().contains(&cap1_id), 6);
        assert!(trail.roles().issued_capabilities().contains(&cap2_id), 7);

        // Verify capabilities have unique IDs
        assert!(cap1_id != cap2_id, 8);

        transfer::public_transfer(cap2, user2);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);

        cap2_id
    };

    ts::end(scenario);
}

#[test]
fun test_revoke_capability() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

    let mut scenario = ts::begin(admin_user);

    let _trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Issue two capabilities
    ts::next_tx(&mut scenario, admin_user);
    let (cap1_id, cap2_id) = {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap1 = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );
        let cap1_id = object::id(&cap1);
        transfer::public_transfer(cap1, user1);

        let cap2 = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );
        let cap2_id = object::id(&cap2);
        transfer::public_transfer(cap2, user2);

        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);

        (cap1_id, cap2_id)
    };

    // Test: Revoke first capability and verify it's removed from tracking
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);
        let cap1 = ts::take_from_address<Capability>(&scenario, user1);

        // Verify both capabilities are tracked before revocation
        let cap_count_before = trail.roles().issued_capabilities().size();
        assert!(trail.roles().issued_capabilities().contains(&cap1_id), 0);
        assert!(trail.roles().issued_capabilities().contains(&cap2_id), 1);

        // Revoke the capability
        trail
            .roles_mut()
            .revoke_capability(
                &admin_cap,
                cap1.id(),
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify capability was removed from tracking
        assert!(trail.roles().issued_capabilities().size() == cap_count_before - 1, 2);
        assert!(!trail.roles().issued_capabilities().contains(&cap1_id), 3);
        // Verify other capability is still tracked
        assert!(trail.roles().issued_capabilities().contains(&cap2_id), 4);

        ts::return_to_address(user1, cap1);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Verify revoked capability object still exists (just invalidated)
    ts::next_tx(&mut scenario, user1);
    {
        assert!(ts::has_most_recent_for_sender<Capability>(&scenario), 5);
    };

    // Test: Revoke second capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);
        let cap2 = ts::take_from_address<Capability>(&scenario, user2);

        let cap_count_before = trail.roles().issued_capabilities().size();

        trail
            .roles_mut()
            .revoke_capability(
                &admin_cap,
                cap2.id(),
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify capability was removed from tracking
        assert!(trail.roles().issued_capabilities().size() == cap_count_before - 1, 6);
        assert!(!trail.roles().issued_capabilities().contains(&cap2_id), 7);

        ts::return_to_address(user2, cap2);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
fun test_destroy_capability() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

    let mut scenario = ts::begin(admin_user);

    let _trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Issue two capabilities
    ts::next_tx(&mut scenario, admin_user);
    let (cap1_id, cap2_id) = {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap1 = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );
        let cap1_id = object::id(&cap1);
        transfer::public_transfer(cap1, user1);

        let cap2 = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );
        let cap2_id = object::id(&cap2);
        transfer::public_transfer(cap2, user2);

        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);

        (cap1_id, cap2_id)
    };

    // User1 destroys their capability
    ts::next_tx(&mut scenario, user1);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let cap1 = ts::take_from_sender<Capability>(&scenario);

        // Verify both capabilities are tracked before destruction
        let cap_count_before = trail.roles().issued_capabilities().size();
        assert!(trail.roles().issued_capabilities().contains(&cap1_id), 0);
        assert!(trail.roles().issued_capabilities().contains(&cap2_id), 1);

        // Destroy the capability
        trail.roles_mut().destroy_capability(cap1);

        // Verify capability was removed from tracking
        assert!(trail.roles().issued_capabilities().size() == cap_count_before - 1, 2);
        assert!(!trail.roles().issued_capabilities().contains(&cap1_id), 3);

        // Verify other capability is still tracked
        assert!(trail.roles().issued_capabilities().contains(&cap2_id), 4);

        ts::return_shared(trail);
    };

    // Verify destroyed capability no longer exists
    ts::next_tx(&mut scenario, user1);
    {
        assert!(!ts::has_most_recent_for_sender<Capability>(&scenario), 5);
    };

    // Test: User2 destroys their own capability
    ts::next_tx(&mut scenario, user2);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let cap2 = ts::take_from_sender<Capability>(&scenario);

        let cap_count_before = trail.roles().issued_capabilities().size();

        trail.roles_mut().destroy_capability(cap2);

        // Verify capability was removed from tracking
        assert!(trail.roles().issued_capabilities().size() == cap_count_before - 1, 6);
        assert!(!trail.roles().issued_capabilities().contains(&cap2_id), 7);

        ts::return_shared(trail);
    };

    // Verify only admin capability remains
    ts::next_tx(&mut scenario, admin_user);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Only the initial admin capability should remain
        assert!(trail.roles().issued_capabilities().size() == 1, 8);

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
    let _trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Create an additional RoleAdmin role
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Initially only admin cap should be tracked
        assert!(trail.roles().issued_capabilities().size() == 1, 0);

        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RoleAdmin"),
                permission::role_admin_permissions(),
                &clock,
                ts::ctx(&mut scenario),
            );

        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Issue capabilities
    ts::next_tx(&mut scenario, admin_user);
    let (record_cap_id, role_cap_id) = {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let record_cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );
        let record_cap_id = object::id(&record_cap);
        transfer::public_transfer(record_cap, record_admin_user);

        let role_cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RoleAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );
        let role_cap_id = object::id(&role_cap);
        transfer::public_transfer(role_cap, role_admin_user);

        // Verify all capabilities are tracked
        assert!(trail.roles().issued_capabilities().size() == 3, 1); // admin + record + role
        assert!(trail.roles().issued_capabilities().contains(&record_cap_id), 2);
        assert!(trail.roles().issued_capabilities().contains(&role_cap_id), 3);

        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);

        (record_cap_id, role_cap_id)
    };

    // Use RecordAdmin capability to add a record
    ts::next_tx(&mut scenario, record_admin_user);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);

        clock.set_for_testing(test_utils::initial_time_for_testing() + 1000);

        let test_data = test_utils::new_test_data(1, b"Test record");
        trail.add_record(
            &record_cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
    };

    // RecordAdmin destroys their capability
    ts::next_tx(&mut scenario, record_admin_user);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        trail.roles_mut().destroy_capability(record_cap);

        // Verify capability was removed
        assert!(trail.roles().issued_capabilities().size() == 2, 4); // admin + role
        assert!(!trail.roles().issued_capabilities().contains(&record_cap_id), 5);

        ts::return_shared(trail);
    };

    // Admin revokes RoleAdmin capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);
        let role_cap = ts::take_from_address<Capability>(&scenario, role_admin_user);

        trail
            .roles_mut()
            .revoke_capability(
                &admin_cap,
                role_cap.id(),
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify capability was removed
        assert!(trail.roles().issued_capabilities().size() == 1, 6); // only admin remains
        assert!(!trail.roles().issued_capabilities().contains(&role_cap_id), 7);

        ts::return_to_address(role_admin_user, role_cap);

        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test, expected_failure(abort_code = audit_trail::role_map::ECapabilityIssuedToMismatch)]
fun test_capability_issued_to_only() {
    let admin_user = @0xAD;
    let authorized_user = @0xB0B;
    let unauthorized_user = @0xCAB;

    let mut scenario = ts::begin(admin_user);

    let _trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Issue capability restricted to authorized_user only
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap = trail
            .roles_mut()
            .new_capability_for_address(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                authorized_user,
                std::option::none(), // no time restriction
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify capability properties
        assert!(cap.issued_to() == std::option::some(authorized_user), 0);
        assert!(cap.valid_from().is_none(), 1);
        assert!(cap.valid_until().is_none(), 2);

        transfer::public_transfer(cap, authorized_user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Authorized user can use the capability
    ts::next_tx(&mut scenario, authorized_user);
    {
        let (record_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let test_data = test_utils::new_test_data(1, b"Authorized record");
        trail.add_record(
            &record_cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Transfer the capability to he unauthorized_user to prepare the next test
        transfer::public_transfer(record_cap, unauthorized_user);

        // Cleanup
        iota::clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

// ===== Error Case Tests =====

#[test]
#[expected_failure(abort_code = main::ECapabilityHasBeenRevoked)]
fun test_revoked_capability_cannot_be_used() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
    };

    // Create role and issue capability to user
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            permission::record_admin_permissions(),
            ts::ctx(&mut scenario),
        );

        let user_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(user_cap, user);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Revoke the capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let user_cap = ts::take_from_address<Capability>(&scenario, user);

        trail.revoke_capability(&admin_cap, user_cap.id());

        ts::return_to_address(user, user_cap);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Try to use revoked capability - should fail
    ts::next_tx(&mut scenario, user);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let user_cap = ts::take_from_sender<Capability>(&scenario);

        clock.set_for_testing(test_utils::initial_time_for_testing() + 1000);

        trail.add_record(
            &user_cap,
            test_utils::new_test_data(1, b"Should fail"),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        iota::clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, user_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::ERoleDoesNotExist)]
fun test_new_capability_for_nonexistent_role() {
    let admin_user = @0xAD;

    let mut scenario = ts::begin(admin_user);

    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
    };

    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let bad_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NonExistentRole"),
            ts::ctx(&mut scenario),
        );

        bad_cap.destroy_for_testing();
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
fun test_revoke_capability_permission_denied() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

    let mut scenario = ts::begin(admin_user);

    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
    };

    // Create two roles: one without revoke permission, one with record permissions
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(&admin_cap, string::utf8(b"NoRevokePerm"), perms, ts::ctx(&mut scenario));

        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            permission::record_admin_permissions(),
            ts::ctx(&mut scenario),
        );

        let user1_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NoRevokePerm"),
            ts::ctx(&mut scenario),
        );

        let user2_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(user1_cap, user1);
        transfer::public_transfer(user2_cap, user2);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // User1 (without revoke permission) tries to revoke User2's capability
    ts::next_tx(&mut scenario, user1);
    {
        let user1_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let user2_cap = ts::take_from_address<Capability>(&scenario, user2);

        trail.revoke_capability(&user1_cap, user2_cap.id());

        ts::return_to_address(user2, user2_cap);
        ts::return_to_sender(&scenario, user1_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
fun test_new_capability_permission_denied() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
    };

    // Create role without add_capabilities permission
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(&admin_cap, string::utf8(b"NoCapPerm"), perms, ts::ctx(&mut scenario));

        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            permission::record_admin_permissions(),
            ts::ctx(&mut scenario),
        );

        let user_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NoCapPerm"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(user_cap, user);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // User tries to issue a new capability without permission
    ts::next_tx(&mut scenario, user);
    {
        let user_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let new_cap = trail.new_capability(
            &user_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );

        new_cap.destroy_for_testing();
        ts::return_to_sender(&scenario, user_cap);
        ts::return_shared(trail);
    };

    // Unauthorized user cannot use the capability
    ts::next_tx(&mut scenario, unauthorized_user);
    {
        let (record_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // This should fail as unauthorized_user has the wrong address
        let test_data = test_utils::new_test_data(1, b"Unauthorized record");
        trail.add_record(
            &record_cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
    };

    ts::end(scenario);
}

/// Test capability with only valid_from restriction (time-restricted from a point).
///
/// This test validates:
/// - Capability can be used after valid_from timestamp
/// - Capability is not restricted by address or end time
/// - Capability cannot be used before valid_from timestamp
#[test, expected_failure(abort_code = audit_trail::role_map::ECapabilityTimeConstraintsNotMet)]
fun test_capability_valid_from_only() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    let valid_from_time = test_utils::initial_time_for_testing() + 5000;

    // Setup
    let _trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Issue capability with valid_from restriction only
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap = trail
            .roles_mut()
            .new_capability(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                std::option::none(), // no address restriction
                std::option::some(valid_from_time),
                std::option::none(), // no valid_until
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify capability properties
        assert!(cap.issued_to().is_none(), 0);
        assert!(cap.valid_from() == std::option::some(valid_from_time), 1);
        assert!(cap.valid_until().is_none(), 2);

        transfer::public_transfer(cap, user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Use the capability after valid_from
    ts::next_tx(&mut scenario, user);
    {
        let (cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(test_utils::initial_time_for_testing() + 6000);

        let test_data = test_utils::new_test_data(1, b"Test record after valid_from");
        trail.add_record(
            &cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, cap, trail, clock);
    };

    // Try to use the capability before valid_from
    ts::next_tx(&mut scenario, user);
    {
        let (cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(test_utils::initial_time_for_testing() + 1000);

        // This should fail as the capability is not valid yet
        let test_data = test_utils::new_test_data(1, b"Test record before valid_from");
        trail.add_record(
            &cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, cap, trail, clock);
    };

    ts::end(scenario);
}

/// Test capability with only valid_until restriction (time-restricted until a point).
///
/// This test validates:
/// - Capability can be used before valid_until timestamp
/// - Capability is not restricted by address or start time
/// - Capability cannot be used after valid_until timestamp
#[test, expected_failure(abort_code = audit_trail::role_map::ECapabilityTimeConstraintsNotMet)]
fun test_capability_valid_until_only() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    let valid_until_time_secs = test_utils::initial_time_for_testing() / 1000 + 10;

    // Setup
    let _trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Issue capability with valid_until restriction
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap = trail
            .roles_mut()
            .new_capability_valid_until(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                valid_until_time_secs,
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify capability properties
        assert!(cap.issued_to().is_none(), 0);
        assert!(cap.valid_from().is_none(), 1);
        assert!(cap.valid_until() == std::option::some(valid_until_time_secs), 2);

        transfer::public_transfer(cap, user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Use the capability before valid_until
    ts::next_tx(&mut scenario, user);
    {
        let (cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(valid_until_time_secs* 1000 - 1000);

        let test_data = test_utils::new_test_data(1, b"Test record before valid_until");
        trail.add_record(
            &cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, cap, trail, clock);
    };

    // Try to use the capability after valid_until
    ts::next_tx(&mut scenario, user);
    {
        let (cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(valid_until_time_secs* 1000 + 1000);

        // This should fail as the capability has expired
        let test_data = test_utils::new_test_data(1, b"Test record after valid_until");
        trail.add_record(
            &cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, cap, trail, clock);
    };

    ts::end(scenario);
}

/// Test capability with valid_from and valid_until restrictions (time window).
///
/// This test validates:
/// - Capability can be used between valid_from and valid_until
/// - Capability is not restricted by address
#[test]
fun test_capability_time_window() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    let valid_from_time = test_utils::initial_time_for_testing() + 5000;
    let valid_until_time = test_utils::initial_time_for_testing() + 10000;

    // Setup
    let _trail_id = setup_trail_with_record_admin_capability_and_time_window_restriction(
        &mut scenario,
        admin_user,
        user,
        valid_from_time / 1000,
        valid_until_time / 1000,
    );

    // Use the capability within the valid time window
    ts::next_tx(&mut scenario, user);
    {
        let (cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(valid_from_time + 2500);

        let test_data = test_utils::new_test_data(1, b"Test record within time window");
        trail.add_record(
            &cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, cap, trail, clock);
    };

    ts::end(scenario);
}

/// Test capability with valid_from and valid_until restrictions (time window).
///
/// This test validates:
/// - Capability cannot be used before valid_from
#[test, expected_failure(abort_code = audit_trail::role_map::ECapabilityTimeConstraintsNotMet)]
fun test_capability_time_window_before_valid_from() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    let valid_from_time_secs = test_utils::initial_time_for_testing() / 1000 + 5;
    let valid_until_time_secs = test_utils::initial_time_for_testing() / 1000 + 10;

    // Setup
    let _trail_id = setup_trail_with_record_admin_capability_and_time_window_restriction(
        &mut scenario,
        admin_user,
        user,
        valid_from_time_secs,
        valid_until_time_secs,
    );

    // Use the capability before valid_from
    ts::next_tx(&mut scenario, user);
    {
        let (cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(valid_from_time_secs* 1000 - 1000);

        let test_data = test_utils::new_test_data(1, b"Test record before valid_from");
        trail.add_record(
            &cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, cap, trail, clock);
    };

    ts::end(scenario);
}

/// Test capability with valid_from and valid_until restrictions (time window).
///
/// This test validates:
/// - Capability cannot be used after valid_until
#[test, expected_failure(abort_code = audit_trail::role_map::ECapabilityTimeConstraintsNotMet)]
fun test_capability_time_window_after_valid_until() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    let valid_from_time_secs = test_utils::initial_time_for_testing() / 1000 + 5;
    let valid_until_time_secs = test_utils::initial_time_for_testing() / 1000 + 10;

    // Setup
    let _trail_id = setup_trail_with_record_admin_capability_and_time_window_restriction(
        &mut scenario,
        admin_user,
        user,
        valid_from_time_secs,
        valid_until_time_secs,
    );

    // Use the capability after valid_until
    ts::next_tx(&mut scenario, user);
    {
        let (cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(valid_until_time_secs* 1000 + 1000);

        let test_data = test_utils::new_test_data(1, b"Test record after valid_until");
        trail.add_record(
            &cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, cap, trail, clock);
    };

    ts::end(scenario);
}

/// Test Capability::is_valid_for_timestamp function.
///
/// This test validates:
/// - Returns true when timestamp is within valid range
/// - Returns false when timestamp is before valid_from
/// - Returns false when timestamp is after valid_until
/// - Returns true when no time restrictions exist
#[test]
fun test_is_valid_for_timestamp() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    let base_time = test_utils::initial_time_for_testing();
    let valid_from_time = base_time + 5000;
    let valid_until_time = base_time + 10000;

    // Setup
    let _trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Test with time-restricted capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap = trail
            .roles_mut()
            .new_capability(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                std::option::none(),
                std::option::some(valid_from_time),
                std::option::some(valid_until_time),
                &clock,
                ts::ctx(&mut scenario),
            );

        // Before valid_from
        assert!(!cap.is_valid_for_timestamp(valid_from_time - 1), 0);

        // At valid_from (inclusive)
        assert!(cap.is_valid_for_timestamp(valid_from_time), 1);

        // During validity period
        assert!(cap.is_valid_for_timestamp(valid_from_time + 2500), 2);

        // Before valid_until (exclusive)
        assert!(cap.is_valid_for_timestamp(valid_until_time - 1), 3);

        // At valid_until (exclusive)
        assert!(!cap.is_valid_for_timestamp(valid_until_time), 4);

        // After valid_until
        assert!(!cap.is_valid_for_timestamp(valid_until_time + 1), 5);

        transfer::public_transfer(cap, user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Test with unrestricted capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let unrestricted_cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );

        // Should be valid at any timestamp
        assert!(unrestricted_cap.is_valid_for_timestamp(0), 6);
        assert!(unrestricted_cap.is_valid_for_timestamp(base_time), 7);
        assert!(unrestricted_cap.is_valid_for_timestamp(valid_until_time + 99999), 8);

        transfer::public_transfer(unrestricted_cap, user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    ts::end(scenario);
}

/// Test Capability::is_currently_valid function.
///
/// This test validates:
/// - Returns true when current time is within valid range
/// - Returns false when current time is outside valid range
/// - Works correctly with Clock object
#[test]
fun test_is_currently_valid() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    let base_time = test_utils::initial_time_for_testing();
    let valid_from_time = base_time + 5000;
    let valid_until_time = base_time + 10000;

    // Setup
    let _trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Issue time-restricted capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap = trail
            .roles_mut()
            .new_capability(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                std::option::none(),
                std::option::some(valid_from_time / 1000),
                std::option::some(valid_until_time / 1000),
                &clock,
                ts::ctx(&mut scenario),
            );

        transfer::public_transfer(cap, user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Test before valid_from
    ts::next_tx(&mut scenario, user);
    {
        let cap = ts::take_from_sender<Capability>(&scenario);
        let mut clock = iota::clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(valid_from_time - 1000);

        assert!(!cap.is_currently_valid(&clock), 0);

        iota::clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, cap);
    };

    // Test during valid period
    ts::next_tx(&mut scenario, user);
    {
        let cap = ts::take_from_sender<Capability>(&scenario);
        let mut clock = iota::clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(valid_from_time + 2500);

        assert!(cap.is_currently_valid(&clock), 1);

        iota::clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, cap);
    };

    // Test after valid_until
    ts::next_tx(&mut scenario, user);
    {
        let cap = ts::take_from_sender<Capability>(&scenario);
        let mut clock = iota::clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(valid_until_time + 1000);

        assert!(!cap.is_currently_valid(&clock), 2);

        iota::clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, cap);
    };

    ts::end(scenario);
}

/// Test Capability::new_capability_without_restrictions function.
///
/// This test validates:
/// - Creates capability with no restrictions
/// - issued_to, valid_from, and valid_until are all None
/// - Capability can be used by anyone at any time
#[test]
fun test_new_capability_without_restrictions() {
    let admin_user = @0xAD;
    let any_user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    // Setup
    let trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Create unrestricted capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify no restrictions
        assert!(cap.issued_to().is_none(), 0);
        assert!(cap.valid_from().is_none(), 1);
        assert!(cap.valid_until().is_none(), 2);
        assert!(cap.role() == string::utf8(b"RecordAdmin"), 3);
        assert!(cap.security_vault_id() == trail_id, 4);

        transfer::public_transfer(cap, any_user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Verify any user can use it at any time
    ts::next_tx(&mut scenario, any_user);
    {
        let (cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(999999999);

        let test_data = test_utils::new_test_data(1, b"Test");
        trail.add_record(
            &cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, cap, trail, clock);
    };

    ts::end(scenario);
}

/// Test Capability::new_capability_valid_until function.
///
/// This test validates:
/// - Creates capability with only valid_until restriction
/// - issued_to and valid_from are None
/// - Capability expires at the specified timestamp
#[test]
fun test_new_capability_valid_until() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    let valid_until_time = test_utils::initial_time_for_testing() + 10000;

    // Setup
    let trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Create capability with valid_until
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap = trail
            .roles_mut()
            .new_capability_valid_until(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                valid_until_time,
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify restrictions
        assert!(cap.issued_to().is_none(), 0);
        assert!(cap.valid_from().is_none(), 1);
        assert!(cap.valid_until() == std::option::some(valid_until_time), 2);
        assert!(cap.role() == string::utf8(b"RecordAdmin"), 3);
        assert!(cap.security_vault_id() == trail_id, 4);

        transfer::public_transfer(cap, user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    ts::end(scenario);
}

/// Test Capability::new_capability_for_address with None for valid_until.
///
/// This test validates:
/// - Creates capability restricted to specific address
/// - valid_until is None (no expiration)
/// - valid_from is None
#[test]
fun test_new_capability_for_address_no_expiration() {
    let admin_user = @0xAD;
    let authorized_user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    // Setup
    let trail_id = setup_trail_with_record_admin_role(&mut scenario, admin_user);

    // Create capability for address without expiration
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let cap = trail
            .roles_mut()
            .new_capability_for_address(
                &admin_cap,
                &string::utf8(b"RecordAdmin"),
                authorized_user,
                std::option::none(), // no expiration
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify restrictions
        assert!(cap.issued_to() == std::option::some(authorized_user), 0);
        assert!(cap.valid_from().is_none(), 1);
        assert!(cap.valid_until().is_none(), 2);
        assert!(cap.role() == string::utf8(b"RecordAdmin"), 3);
        assert!(cap.security_vault_id() == trail_id, 4);

        transfer::public_transfer(cap, authorized_user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    ts::end(scenario);
}
