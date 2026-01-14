#[test_only]
module audit_trail::capability_tests;

use audit_trail::{
    capability::Capability,
    locking,
    main::{Self, AuditTrail},
    permission,
    test_utils::{Self, TestData, setup_test_audit_trail}
};
use iota::test_scenario as ts;
use std::string;

#[test]
fun test_new_capability() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

    let mut scenario = ts::begin(admin_user);

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

    // Create a role to issue capabilities for
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            permission::record_admin_permissions(),
            ts::ctx(&mut scenario),
        );

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Issue first capability and verify it's tracked
    ts::next_tx(&mut scenario, admin_user);
    let cap1_id = {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let initial_cap_count = trail.issued_capabilities().size();
        assert!(initial_cap_count == 1, 0); // Only admin cap

        let cap1 = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );

        assert!(cap1.role() == string::utf8(b"RecordAdmin"), 1);
        assert!(cap1.trail_id() == trail_id, 2);

        let cap1_id = object::id(&cap1);
        assert!(trail.issued_capabilities().size() == initial_cap_count + 1, 3);
        assert!(trail.issued_capabilities().contains(&cap1_id), 4);

        transfer::public_transfer(cap1, user1);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);

        cap1_id
    };

    // Issue second capability and verify both are tracked with unique IDs
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let previous_cap_count = trail.issued_capabilities().size();

        let cap2 = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );

        let cap2_id = object::id(&cap2);

        assert!(trail.issued_capabilities().size() == previous_cap_count + 1, 5);
        assert!(trail.issued_capabilities().contains(&cap1_id), 6);
        assert!(trail.issued_capabilities().contains(&cap2_id), 7);
        assert!(cap1_id != cap2_id, 8);

        transfer::public_transfer(cap2, user2);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_revoke_capability() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

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

    // Create role
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            permission::record_admin_permissions(),
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

    // Revoke first capability and verify it's removed from tracking
    ts::next_tx(&mut scenario, user1);
    {
        let admin_cap = ts::take_from_address<Capability>(&scenario, admin_user);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let cap1 = ts::take_from_sender<Capability>(&scenario);

        let cap_count_before = trail.issued_capabilities().size();
        assert!(trail.issued_capabilities().contains(&cap1_id), 0);
        assert!(trail.issued_capabilities().contains(&cap2_id), 1);

        trail.revoke_capability(&admin_cap, cap1.id());

        assert!(trail.issued_capabilities().size() == cap_count_before - 1, 2);
        assert!(!trail.issued_capabilities().contains(&cap1_id), 3);
        assert!(trail.issued_capabilities().contains(&cap2_id), 4);

        ts::return_to_address(admin_user, admin_cap);
        ts::return_to_sender(&scenario, cap1);
        ts::return_shared(trail);
    };

    // Verify revoked capability object still exists (just invalidated)
    ts::next_tx(&mut scenario, user1);
    {
        assert!(ts::has_most_recent_for_sender<Capability>(&scenario), 5);
    };

    ts::end(scenario);
}

#[test]
fun test_destroy_capability() {
    let admin_user = @0xAD;
    let user1 = @0xB0B;
    let user2 = @0xCAB;

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

    // Create role
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            permission::record_admin_permissions(),
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

    // User1 destroys their capability
    ts::next_tx(&mut scenario, user1);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let cap1 = ts::take_from_sender<Capability>(&scenario);

        let cap_count_before = trail.issued_capabilities().size();
        assert!(trail.issued_capabilities().contains(&cap1_id), 0);
        assert!(trail.issued_capabilities().contains(&cap2_id), 1);

        trail.destroy_capability(cap1);

        assert!(trail.issued_capabilities().size() == cap_count_before - 1, 2);
        assert!(!trail.issued_capabilities().contains(&cap1_id), 3);
        assert!(trail.issued_capabilities().contains(&cap2_id), 4);

        ts::return_shared(trail);
    };

    // Verify destroyed capability no longer exists
    ts::next_tx(&mut scenario, user1);
    {
        assert!(!ts::has_most_recent_for_sender<Capability>(&scenario), 5);
    };

    ts::end(scenario);
}

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
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

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

        let mut clock = iota::clock::create_for_testing(ts::ctx(&mut scenario));
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

    ts::end(scenario);
}
