#[test_only]
module audit_trail::role_tests;

use audit_trail::{
    locking,
    main::initial_admin_role_name,
    permission,
    test_utils::{
        Self,
        setup_test_audit_trail,
        fetch_capability_trail_and_clock,
        cleanup_capability_trail_and_clock
    }
};
use iota::test_scenario as ts;
use std::string;

/// Test comprehensive role-based access control delegation workflow.
///
/// This test validates the complete permission delegation chain:
/// 1. An admin user creates an audit trail with full admin permissions
/// 2. Admin creates two specialized roles: RoleAdmin (for role management) and CapAdmin (for capability management)
/// 3. Admin delegates these roles to different users by issuing capabilities
/// 4. RoleAdmin user leverages their permissions to create a RecordAdmin role
/// 5. CapAdmin user leverages their permissions to issue a RecordAdmin capability
/// 6. RecordAdmin user uses their capability to add a record to the audit trail
///
/// This test ensures:
/// - Role creation works correctly with specific permission sets
/// - Capability issuance and transfer functions properly
/// - Permission delegation cascade works (Admin -> RoleAdmin -> RecordAdmin)
/// - Permission delegation cascade works (Admin -> CapAdmin -> RecordAdmin capability)
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
        assert!(admin_cap.security_vault_id() == trail_id, 1);

        // Transfer the admin capability to the user
        transfer::public_transfer(admin_cap, admin_user);

        trail_id
    };

    // Step 2: Admin creates RoleAdmin and CapAdmin roles
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Verify initial state - should only have the initial admin role
        assert!(trail.roles().size() == 1, 2);

        // Create RoleAdmin role
        let role_admin_perms = permission::role_admin_permissions();
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RoleAdmin"),
                role_admin_perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        // Create CapAdmin role
        let cap_admin_perms = permission::cap_admin_permissions();
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"CapAdmin"),
                cap_admin_perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify both roles were created
        assert!(trail.roles().size() == 3, 3); // Initial admin + RoleAdmin + CapAdmin
        assert!(trail.roles().has_role(&string::utf8(b"RoleAdmin")), 4);
        assert!(trail.roles().has_role(&string::utf8(b"CapAdmin")), 5);

        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Step 3: Admin creates capability for RoleAdmin and CapAdmin and transfers to the respective users
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let role_admin_cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"RoleAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify the capability was created with correct role and trail ID
        assert!(role_admin_cap.role() == string::utf8(b"RoleAdmin"), 6);
        assert!(role_admin_cap.security_vault_id() == trail_id, 7);

        iota::transfer::public_transfer(role_admin_cap, role_admin_user);

        let cap_admin_cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"CapAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify the capability was created with correct role and trail ID
        assert!(cap_admin_cap.role() == string::utf8(b"CapAdmin"), 8);
        assert!(cap_admin_cap.security_vault_id() == trail_id, 9);

        iota::transfer::public_transfer(cap_admin_cap, cap_admin_user);

        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Step 5: RoleAdmin creates RecordAdmin role (demonstrating delegated role management)
    ts::next_tx(&mut scenario, role_admin_user);
    {
        let (role_admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Verify RoleAdmin has the correct role
        assert!(role_admin_cap.role() == string::utf8(b"RoleAdmin"), 10);

        let record_admin_perms = permission::record_admin_permissions();
        trail
            .roles_mut()
            .create_role(
                &role_admin_cap,
                string::utf8(b"RecordAdmin"),
                record_admin_perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify RecordAdmin role was created successfully
        assert!(trail.roles().size() == 4, 11); // Initial admin + RoleAdmin + CapAdmin + RecordAdmin
        assert!(trail.roles().has_role(&string::utf8(b"RecordAdmin")), 12);

        cleanup_capability_trail_and_clock(&scenario, role_admin_cap, trail, clock);
    };

    // Step 6: CapAdmin creates capability for RecordAdmin and transfers to record_admin_user
    ts::next_tx(&mut scenario, cap_admin_user);
    {
        let (cap_admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Verify CapAdmin has the correct role
        assert!(cap_admin_cap.role() == string::utf8(b"CapAdmin"), 13);

        let record_admin_cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &cap_admin_cap,
                &string::utf8(b"RecordAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );

        // Verify the capability was created with correct role and trail ID
        assert!(record_admin_cap.role() == string::utf8(b"RecordAdmin"), 14);
        assert!(record_admin_cap.security_vault_id() == trail_id, 15);

        iota::transfer::public_transfer(record_admin_cap, record_admin_user);

        cleanup_capability_trail_and_clock(&scenario, cap_admin_cap, trail, clock);
    };

    // Step 7: RecordAdmin adds a new record to the audit trail (demonstrating delegated record management)
    ts::next_tx(&mut scenario, record_admin_user);
    {
        let (record_admin_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(
            &mut scenario,
        );
        clock.set_for_testing(test_utils::initial_time_for_testing() + 1000);

        // Verify RecordAdmin has the correct role
        assert!(record_admin_cap.role() == string::utf8(b"RecordAdmin"), 16);

        // Verify initial record count
        let initial_record_count = trail.records().length();

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

        cleanup_capability_trail_and_clock(&scenario, record_admin_cap, trail, clock);
    };

    // Cleanup
    ts::next_tx(&mut scenario, admin_user);
    ts::end(scenario);
}
