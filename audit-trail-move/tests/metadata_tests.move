#[test_only]
module audit_trail::metadata_tests;

use audit_trail::{
    capability::Capability,
    locking,
    main::{Self, AuditTrail},
    permission,
    test_utils::{TestData, setup_test_audit_trail}
};
use iota::test_scenario as ts;
use std::string;

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
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Create MetadataAdmin role with metadata permissions
        let metadata_perms = permission::metadata_admin_permissions();
        trail.create_role(
            &admin_cap,
            string::utf8(b"MetadataAdmin"),
            metadata_perms,
            ts::ctx(&mut scenario),
        );

        // Issue capability to metadata admin user
        let metadata_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"MetadataAdmin"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(metadata_cap, metadata_admin_user);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Test: MetadataAdmin updates metadata
    ts::next_tx(&mut scenario, metadata_admin_user);
    {
        let metadata_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Update metadata
        let new_metadata = std::option::some(string::utf8(b"Updated metadata value"));
        trail.update_metadata(
            &metadata_cap,
            new_metadata,
            ts::ctx(&mut scenario),
        );

        // Verify metadata was updated
        let current_metadata = trail.metadata();
        assert!(current_metadata.is_some(), 0);
        assert!(*current_metadata.borrow() == string::utf8(b"Updated metadata value"), 1);

        ts::return_to_sender(&scenario, metadata_cap);
        ts::return_shared(trail);
    };

    // Test: Update metadata again to verify multiple updates work
    ts::next_tx(&mut scenario, metadata_admin_user);
    {
        let metadata_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Update to different value
        let new_metadata = std::option::some(string::utf8(b"Second update"));
        trail.update_metadata(
            &metadata_cap,
            new_metadata,
            ts::ctx(&mut scenario),
        );

        // Verify metadata was updated
        let current_metadata = trail.metadata();
        assert!(current_metadata.is_some(), 2);
        assert!(*current_metadata.borrow() == string::utf8(b"Second update"), 3);

        ts::return_to_sender(&scenario, metadata_cap);
        ts::return_shared(trail);
    };

    // Test: Set metadata to none
    ts::next_tx(&mut scenario, metadata_admin_user);
    {
        let metadata_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Set to none
        trail.update_metadata(
            &metadata_cap,
            std::option::none(),
            ts::ctx(&mut scenario),
        );

        // Verify metadata is now none
        let current_metadata = trail.metadata();
        assert!(current_metadata.is_none(), 4);

        ts::return_to_sender(&scenario, metadata_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

// ===== Error Case Tests =====

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
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
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Create role with only add_record permission (no update_metadata)
        let perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(
            &admin_cap,
            string::utf8(b"NoMetadataPerm"),
            perms,
            ts::ctx(&mut scenario),
        );

        let user_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NoMetadataPerm"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(user_cap, user);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // User tries to update metadata - should fail
    ts::next_tx(&mut scenario, user);
    {
        let user_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // This should fail - no update_metadata permission
        trail.update_metadata(
            &user_cap,
            std::option::some(string::utf8(b"Should fail")),
            ts::ctx(&mut scenario),
        );

        ts::return_to_sender(&scenario, user_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::ECapabilityHasBeenRevoked)]
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
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Create MetadataAdmin role
        let metadata_perms = permission::metadata_admin_permissions();
        trail.create_role(
            &admin_cap,
            string::utf8(b"MetadataAdmin"),
            metadata_perms,
            ts::ctx(&mut scenario),
        );

        // Issue capability
        let metadata_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"MetadataAdmin"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(metadata_cap, metadata_admin_user);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Revoke the capability
    ts::next_tx(&mut scenario, admin_user);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let metadata_cap = ts::take_from_address<Capability>(&scenario, metadata_admin_user);

        trail.revoke_capability(&admin_cap, metadata_cap.id());

        ts::return_to_address(metadata_admin_user, metadata_cap);
        ts::return_to_sender(&scenario, admin_cap);
        ts::return_shared(trail);
    };

    // Try to use revoked capability - should fail
    ts::next_tx(&mut scenario, metadata_admin_user);
    {
        let metadata_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // This should fail - capability has been revoked
        trail.update_metadata(
            &metadata_cap,
            std::option::some(string::utf8(b"Should fail")),
            ts::ctx(&mut scenario),
        );

        ts::return_to_sender(&scenario, metadata_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}
