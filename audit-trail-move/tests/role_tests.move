#[test_only]
module audit_trail::role_tests;

use audit_trail::{
    capability::Capability,
    locking,
    main::{Self, AuditTrail, initial_admin_role_name},
    permission,
    test_utils::{Self, TestData, setup_test_audit_trail}
};
use iota::{clock, test_scenario as ts};
use std::string;

#[test]
fun test_role_based_permission_delegation() {
    let admin_user = @0xAD;
    let role_admin_user = @0xB0B;
    let cap_admin_user = @0xCAB;
    let record_admin_user = @0xDED;

    let mut scenario = ts::begin(admin_user);

    // Step 1: admin_user creates the audit trail
    let trail_id = {
        let locking_config = locking::new(locking::window_count_based(0));

        let (admin_cap, trail_id) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );

        // Verify admin capability was created with correct role and trail reference
        assert!(admin_cap.role() == initial_admin_role_name(), 0);
        assert!(admin_cap.trail_id() == trail_id, 1);

        // Transfer the admin capability to the user
        transfer::public_transfer(admin_cap, admin_user);

        trail_id
    };

    // Step 2: Admin creates RoleAdmin and CapAdmin roles
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Verify initial state - should only have the initial admin role
        assert!(trail.roles().size() == 1, 2);

        // Create RoleAdmin role
        let role_admin_perms = permission::role_admin_permissions();
        trail.create_role(
            &admin_cap,
            string::utf8(b"RoleAdmin"),
            role_admin_perms,
            ts::ctx(&mut scenario),
        );

        // Create CapAdmin role
        let cap_admin_perms = permission::cap_admin_permissions();
        trail.create_role(
            &admin_cap,
            string::utf8(b"CapAdmin"),
            cap_admin_perms,
            ts::ctx(&mut scenario),
        );

        // Verify both roles were created
        assert!(trail.roles().size() == 3, 3); // Initial admin + RoleAdmin + CapAdmin
        assert!(trail.has_role(&string::utf8(b"RoleAdmin")), 4);
        assert!(trail.has_role(&string::utf8(b"CapAdmin")), 5);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Step 3: Admin creates capability for RoleAdmin and CapAdmin and transfers to the respective users
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let role_admin_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RoleAdmin"),
            ts::ctx(&mut scenario),
        );

        // Verify the capability was created with correct role and trail ID
        assert!(role_admin_cap.role() == string::utf8(b"RoleAdmin"), 6);
        assert!(role_admin_cap.trail_id() == trail_id, 7);

        iota::transfer::public_transfer(role_admin_cap, role_admin_user);

        let cap_admin_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"CapAdmin"),
            ts::ctx(&mut scenario),
        );

        // Verify the capability was created with correct role and trail ID
        assert!(cap_admin_cap.role() == string::utf8(b"CapAdmin"), 8);
        assert!(cap_admin_cap.trail_id() == trail_id, 9);

        iota::transfer::public_transfer(cap_admin_cap, cap_admin_user);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Step 5: RoleAdmin creates RecordAdmin role (demonstrating delegated role management)
    ts::next_tx(&mut scenario, role_admin_user);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let role_admin_cap = ts::take_from_sender<Capability>(&scenario);

        // Verify RoleAdmin has the correct role
        assert!(role_admin_cap.role() == string::utf8(b"RoleAdmin"), 10);

        let record_admin_perms = permission::record_admin_permissions();
        trail.create_role(
            &role_admin_cap,
            string::utf8(b"RecordAdmin"),
            record_admin_perms,
            ts::ctx(&mut scenario),
        );

        // Verify RecordAdmin role was created successfully
        assert!(trail.roles().size() == 4, 11); // Initial admin + RoleAdmin + CapAdmin + RecordAdmin
        assert!(trail.has_role(&string::utf8(b"RecordAdmin")), 12);

        ts::return_to_sender(&scenario, role_admin_cap);
        ts::return_shared(trail);
    };

    // Step 6: CapAdmin creates capability for RecordAdmin and transfers to record_admin_user
    ts::next_tx(&mut scenario, cap_admin_user);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let cap_admin_cap = ts::take_from_sender<Capability>(&scenario);

        // Verify CapAdmin has the correct role
        assert!(cap_admin_cap.role() == string::utf8(b"CapAdmin"), 13);

        let record_admin_cap = trail.new_capability(
            &cap_admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );

        // Verify the capability was created with correct role and trail ID
        assert!(record_admin_cap.role() == string::utf8(b"RecordAdmin"), 14);
        assert!(record_admin_cap.trail_id() == trail_id, 15);

        iota::transfer::public_transfer(record_admin_cap, record_admin_user);

        ts::return_to_sender(&scenario, cap_admin_cap);
        ts::return_shared(trail);
    };

    // Step 7: RecordAdmin adds a new record to the audit trail (demonstrating delegated record management)
    ts::next_tx(&mut scenario, record_admin_user);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_admin_cap = ts::take_from_sender<Capability>(&scenario);

        // Verify RecordAdmin has the correct role
        assert!(record_admin_cap.role() == string::utf8(b"RecordAdmin"), 16);

        // Verify initial record count
        let initial_record_count = trail.records().length();

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(test_utils::initial_time_for_testing() + 1000);

        let test_data = test_utils::new_test_data(42, b"Test record added by RecordAdmin");

        trail.add_record(
            &record_admin_cap,
            test_data,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Verify the record was added successfully
        assert!(trail.records().length() == initial_record_count + 1, 17);

        clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, record_admin_cap);
        ts::return_shared(trail);
    };

    // Cleanup
    ts::next_tx(&mut scenario, admin_user);
    ts::end(scenario);
}

#[test]
fun test_delete_role_success() {
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

        // Verify initial state - only Admin role exists
        assert!(trail.roles().size() == 1, 0);

        // Create a role to delete
        let perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(&admin_cap, string::utf8(b"RoleToDelete"), perms, ts::ctx(&mut scenario));

        // Verify the role was created
        assert!(trail.roles().size() == 2, 1);
        assert!(trail.has_role(&string::utf8(b"RoleToDelete")), 2);

        // Delete the role
        trail.delete_role(&admin_cap, &string::utf8(b"RoleToDelete"), ts::ctx(&mut scenario));

        // Verify the role was deleted
        assert!(trail.roles().size() == 1, 3);
        assert!(!trail.has_role(&string::utf8(b"RoleToDelete")), 4);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

// ===== Error Case Tests =====

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
fun test_create_role_permission_denied() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    // Setup
    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
    };

    // Create role without RolesAdd permission
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Create role WITHOUT add_roles permission
        let perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(&admin_cap, string::utf8(b"NoRolesPerm"), perms, ts::ctx(&mut scenario));

        let user_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NoRolesPerm"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(user_cap, user);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // User tries to create a role - should fail
    ts::next_tx(&mut scenario, user);
    {
        let user_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let perms = permission::from_vec(vector[permission::add_record()]);

        // This should fail - no add_roles permission
        trail.create_role(&user_cap, string::utf8(b"NewRole"), perms, ts::ctx(&mut scenario));

        ts::return_to_sender(&scenario, user_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
fun test_delete_role_permission_denied() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    // Setup
    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
    };

    // Create roles
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Create a role to delete
        let perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(&admin_cap, string::utf8(b"RoleToDelete"), perms, ts::ctx(&mut scenario));

        // Create role WITHOUT delete_roles permission
        let no_delete_perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(
            &admin_cap,
            string::utf8(b"NoDeleteRolePerm"),
            no_delete_perms,
            ts::ctx(&mut scenario),
        );

        let user_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NoDeleteRolePerm"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(user_cap, user);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // User tries to delete a role - should fail
    ts::next_tx(&mut scenario, user);
    {
        let user_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // This should fail - no delete_roles permission
        trail.delete_role(&user_cap, &string::utf8(b"RoleToDelete"), ts::ctx(&mut scenario));

        ts::return_to_sender(&scenario, user_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
fun test_update_role_permissions_permission_denied() {
    let admin_user = @0xAD;
    let user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    // Setup
    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
    };

    // Create roles
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Create a role to update
        let perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(&admin_cap, string::utf8(b"RoleToUpdate"), perms, ts::ctx(&mut scenario));

        // Create role WITHOUT update_roles permission
        let no_update_perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(
            &admin_cap,
            string::utf8(b"NoUpdateRolePerm"),
            no_update_perms,
            ts::ctx(&mut scenario),
        );

        let user_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NoUpdateRolePerm"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(user_cap, user);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // User tries to update a role - should fail
    ts::next_tx(&mut scenario, user);
    {
        let user_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let new_perms = permission::from_vec(vector[permission::delete_record()]);

        // This should fail - no update_roles permission
        trail.update_role_permissions(
            &user_cap,
            &string::utf8(b"RoleToUpdate"),
            new_perms,
            ts::ctx(&mut scenario),
        );

        ts::return_to_sender(&scenario, user_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::ERoleDoesNotExist)]
fun test_get_role_permissions_nonexistent() {
    let admin_user = @0xAD;

    let mut scenario = ts::begin(admin_user);

    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin_user);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // This should fail - role doesn't exist
        let _perms = trail.get_role_permissions(&string::utf8(b"NonExistentRole"));

        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_update_role_permissions_success() {
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

        // Create a role with add_record permission
        let initial_perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(
            &admin_cap,
            string::utf8(b"TestRole"),
            initial_perms,
            ts::ctx(&mut scenario),
        );

        // Verify the role was created with add_record permission
        let perms = trail.get_role_permissions(&string::utf8(b"TestRole"));
        assert!(perms.contains(&permission::add_record()), 0);
        assert!(!perms.contains(&permission::delete_record()), 1);

        // Update the role to have delete_record permission instead
        let new_perms = permission::from_vec(vector[permission::delete_record()]);
        trail.update_role_permissions(
            &admin_cap,
            &string::utf8(b"TestRole"),
            new_perms,
            ts::ctx(&mut scenario),
        );

        // Verify the permissions were updated
        let updated_perms = trail.get_role_permissions(&string::utf8(b"TestRole"));
        assert!(!updated_perms.contains(&permission::add_record()), 2);
        assert!(updated_perms.contains(&permission::delete_record()), 3);

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::ERoleDoesNotExist)]
fun test_update_role_permissions_nonexistent() {
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

        let new_perms = permission::from_vec(vector[permission::add_record()]);

        // This should fail - role doesn't exist
        trail.update_role_permissions(
            &admin_cap,
            &string::utf8(b"NonExistentRole"),
            new_perms,
            ts::ctx(&mut scenario),
        );

        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}
