#[allow(lint(abort_without_constant))]
#[test_only]
module audit_trail::metadata_tests;

use audit_trail::{
    locking,
    permission,
    test_utils::{
        setup_test_audit_trail,
        fetch_capability_trail_and_clock,
        cleanup_capability_trail_and_clock
    }
};
use iota::test_scenario as ts;
use std::string;
use tf_components::capability::Capability;

// ===== Success Case Tests =====

#[test]
fun test_update_metadata_success() {
    let admin_user = @0xAD;
    let metadata_admin_user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    // Setup: Create audit trail
    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
    };

    // Create MetadataAdmin role and capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Create MetadataAdmin role with metadata permissions
        let metadata_perms = permission::metadata_admin_permissions();
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"MetadataAdmin"),
                metadata_perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        // Issue capability to metadata admin user
        let metadata_cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"MetadataAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );

        transfer::public_transfer(metadata_cap, metadata_admin_user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Test: MetadataAdmin updates metadata
    ts::next_tx(&mut scenario, metadata_admin_user);
    {
        let (metadata_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Update metadata
        let new_metadata = std::option::some(string::utf8(b"Updated metadata value"));
        trail.update_metadata(
            &metadata_cap,
            new_metadata,
            &clock,
            ts::ctx(&mut scenario),
        );

        // Verify metadata was updated
        let current_metadata = trail.metadata();
        assert!(current_metadata.is_some(), 0);
        assert!(*current_metadata.borrow() == string::utf8(b"Updated metadata value"), 1);

        cleanup_capability_trail_and_clock(&scenario, metadata_cap, trail, clock);
    };

    // Test: Update metadata again to verify multiple updates work
    ts::next_tx(&mut scenario, metadata_admin_user);
    {
        let (metadata_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Update to different value
        let new_metadata = std::option::some(string::utf8(b"Second update"));
        trail.update_metadata(
            &metadata_cap,
            new_metadata,
            &clock,
            ts::ctx(&mut scenario),
        );

        // Verify metadata was updated
        let current_metadata = trail.metadata();
        assert!(current_metadata.is_some(), 2);
        assert!(*current_metadata.borrow() == string::utf8(b"Second update"), 3);

        cleanup_capability_trail_and_clock(&scenario, metadata_cap, trail, clock);
    };

    // Test: Set metadata to none
    ts::next_tx(&mut scenario, metadata_admin_user);
    {
        let (metadata_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Set to none
        trail.update_metadata(
            &metadata_cap,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Verify metadata is now none
        let current_metadata = trail.metadata();
        assert!(current_metadata.is_none(), 4);

        cleanup_capability_trail_and_clock(&scenario, metadata_cap, trail, clock);
    };

    ts::end(scenario);
}

// ===== Error Case Tests =====

#[test]
#[expected_failure(abort_code = audit_trail::role_map::ECapabilityPermissionDenied)]
fun test_update_metadata_permission_denied() {
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

    // Create role WITHOUT update_metadata permission
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Create role with only add_record permission (no update_metadata)
        let perms = permission::from_vec(vector[permission::add_record()]);
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"NoMetadataPerm"),
                perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        let user_cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"NoMetadataPerm"),
                &clock,
                ts::ctx(&mut scenario),
            );

        transfer::public_transfer(user_cap, user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // User tries to update metadata - should fail
    ts::next_tx(&mut scenario, user);
    {
        let (user_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // This should fail - no update_metadata permission
        trail.update_metadata(
            &user_cap,
            std::option::some(string::utf8(b"Should fail")),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, user_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = audit_trail::role_map::ECapabilityHasBeenRevoked)]
fun test_update_metadata_revoked_capability() {
    let admin_user = @0xAD;
    let metadata_admin_user = @0xB0B;

    let mut scenario = ts::begin(admin_user);

    // Setup: Create audit trail
    {
        let locking_config = locking::new(locking::window_count_based(0));
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin_user);
    };

    // Create MetadataAdmin role and capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Create MetadataAdmin role
        let metadata_perms = permission::metadata_admin_permissions();
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"MetadataAdmin"),
                metadata_perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        // Issue capability
        let metadata_cap = trail
            .roles_mut()
            .new_capability_without_restrictions(
                &admin_cap,
                &string::utf8(b"MetadataAdmin"),
                &clock,
                ts::ctx(&mut scenario),
            );

        transfer::public_transfer(metadata_cap, metadata_admin_user);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Revoke the capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);
        let metadata_cap = ts::take_from_address<Capability>(&scenario, metadata_admin_user);

        trail
            .roles_mut()
            .revoke_capability(&admin_cap, metadata_cap.id(), &clock, ts::ctx(&mut scenario));

        ts::return_to_address(metadata_admin_user, metadata_cap);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Try to use revoked capability - should fail
    ts::next_tx(&mut scenario, metadata_admin_user);
    {
        let (metadata_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        // This should fail - capability has been revoked
        trail.update_metadata(
            &metadata_cap,
            std::option::some(string::utf8(b"Should fail")),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, metadata_cap, trail, clock);
    };

    ts::end(scenario);
}
