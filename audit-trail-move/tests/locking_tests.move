#[allow(lint(abort_without_constant))]
#[test_only]
module audit_trail::locking_tests;

use audit_trail::{
    locking,
    main::AuditTrail,
    permission,
    test_utils::{
        Self,
        TestData,
        setup_test_audit_trail,
        new_test_data,
        initial_time_for_testing,
        fetch_capability_trail_and_clock,
        cleanup_capability_trail_and_clock,
        cleanup_trail_and_clock
    }
};
use iota::{clock, test_scenario as ts};
use std::string;
use tf_components::capability::Capability;

// ===== Time-Based Locking Tests =====

#[test]
fun test_time_based_locking_within_window() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with 1 hour time-based locking
    {
        let locking_config = locking::time_based(3600);
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Test")),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));

        // 1 second after creation - locked
        clock.set_for_testing(initial_time_for_testing() + 1000);
        assert!(trail.is_record_locked(0, &clock), 0);

        // 30 minutes after - locked
        clock.set_for_testing(initial_time_for_testing() + 1800 * 1000);
        assert!(trail.is_record_locked(0, &clock), 1);

        // 59 minutes after - locked
        clock.set_for_testing(initial_time_for_testing() + 3540 * 1000);
        assert!(trail.is_record_locked(0, &clock), 2);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_time_based_locking_outside_window() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with 1 hour time-based locking
    {
        let locking_config = locking::time_based(3600);
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Test")),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));

        // 1 hour + 1 second after creation - unlocked
        clock.set_for_testing(initial_time_for_testing() + 3601 * 1000);
        assert!(!trail.is_record_locked(0, &clock), 0);

        // 2 hours after - unlocked
        clock.set_for_testing(initial_time_for_testing() + 7200 * 1000);
        assert!(!trail.is_record_locked(0, &clock), 1);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

// ===== Count-Based Locking Tests =====

#[test]
fun test_count_based_locking() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with count-based locking (last 2 locked)
    {
        let locking_config = locking::count_based(2);
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role and capability
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Add 5 records and verify locking
    ts::next_tx(&mut scenario, admin);
    {
        let record_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let mut i = 0u64;
        while (i < 5) {
            trail.add_record(
                &record_cap,
                new_test_data(i, b"Record"),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );
            i = i + 1;
        };

        // With 5 records and last 2 locked:
        // Records 0, 1, 2 = unlocked (have 4, 3, 2 records after them)
        // Records 3, 4 = locked (have 1, 0 records after them)
        assert!(!trail.is_record_locked(0, &clock), 0);
        assert!(!trail.is_record_locked(1, &clock), 1);
        assert!(!trail.is_record_locked(2, &clock), 2);
        assert!(trail.is_record_locked(3, &clock), 3);
        assert!(trail.is_record_locked(4, &clock), 4);

        clock::destroy_for_testing(clock);
        record_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_count_based_locking_single_record() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with "last 3 locked" - single record should be locked
    {
        let locking_config = locking::count_based(3);
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Single")),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        assert!(trail.is_record_locked(0, &clock), 0);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

// ===== No Locking Tests =====

#[test]
fun test_no_locking() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Test")),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing());

        // No locking config = never locked
        assert!(!trail.is_record_locked(0, &clock), 0);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

// ===== Update Locking Config Tests =====

#[test]
fun test_update_locking_config() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with no locking
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Test")),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create LockingAdmin role
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let perms = permission::from_vec(vector[permission::update_locking_config()]);
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"LockingAdmin"),
                perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        let locking_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"LockingAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(locking_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Update from no-locking to time-based
    ts::next_tx(&mut scenario, admin);
    {
        let (locking_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Initially unlocked
        assert!(!trail.is_record_locked(0, &clock), 0);

        // Update to 1 hour time-based locking
        trail.update_locking_config(
            &locking_cap,
            locking::time_based(3600),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Now locked
        assert!(trail.is_record_locked(0, &clock), 1);

        // locking_cap.destroy_for_testing();
        cleanup_capability_trail_and_clock(&scenario, locking_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = audit_trail::role_map::ECapabilityPermissionDenied)]
fun test_update_locking_config_permission_denied() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create role WITHOUT UpdateLockingConfig permission
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let perms = permission::from_vec(vector[permission::add_record()]);
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"NoLockingPerm"),
                perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        let no_locking_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"NoLockingPerm"),
            &clock,
            ts::ctx(&mut scenario),
        );
        transfer::public_transfer(no_locking_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Try to update locking config - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let (no_locking_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail.update_locking_config(
            &no_locking_cap,
            locking::time_based(3600),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, no_locking_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
fun test_update_locking_config_for_delete_record() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with no locking
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Test")),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create role with UpdateLockingConfigForDeleteRecord permission
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let perms = permission::from_vec(vector[
            permission::update_locking_config_for_delete_record(),
        ]);
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"DeleteLockAdmin"),
                perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        let delete_lock_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"DeleteLockAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(delete_lock_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Update delete_record_lock
    ts::next_tx(&mut scenario, admin);
    {
        let (delete_lock_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(
            &mut scenario,
        );
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Initially unlocked
        assert!(!trail.is_record_locked(0, &clock), 0);

        // Update to count-based (last 5 locked)
        trail.update_locking_config_for_delete_record(
            &delete_lock_cap,
            locking::window_count_based(5),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Now locked (single record, last 5 are locked)
        assert!(trail.is_record_locked(0, &clock), 1);

        cleanup_capability_trail_and_clock(&scenario, delete_lock_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = audit_trail::role_map::ECapabilityPermissionDenied)]
fun test_update_locking_config_for_delete_record_permission_denied() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create role with update_locking_config but NOT update_locking_config_for_delete_record
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let perms = permission::from_vec(vector[permission::update_locking_config()]);
        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"WrongPerm"),
                perms,
                &clock,
                ts::ctx(&mut scenario),
            );

        let wrong_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"WrongPerm"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(wrong_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Try to update delete_record_lock - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let (wrong_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail.update_locking_config_for_delete_record(
            &wrong_cap,
            locking::window_count_based(5),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, wrong_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
fun test_delete_record_after_time_lock_expires() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with 1 hour time-based locking and initial record
    {
        let locking_config = locking::time_based(3600); // 1 hour = 3600 seconds
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Locked record")),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Test boundary: exactly at lock expiry (should still be locked)
    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));

        // Exactly at 1 hour mark - record age equals time window (edge case)
        // So at exactly the boundary, record should be UNLOCKED
        clock.set_for_testing(initial_time_for_testing() + 3600 * 1000);
        assert!(!trail.is_record_locked(0, &clock), 0);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    // Delete record after time lock expires
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        // 1 hour + 1 second after creation - clearly past the lock window
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 3601 * 1000);

        // Verify record exists and is unlocked
        assert!(trail.has_record(0), 1);
        assert!(!trail.is_record_locked(0, &clock), 2);

        // Delete should succeed
        trail.delete_record(&record_cap, 0, &clock, ts::ctx(&mut scenario));

        // Verify record was deleted
        assert!(!trail.has_record(0), 3);

        clock::destroy_for_testing(clock);
        record_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_time_lock_boundary_just_before_expiry() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with 1 hour time-based locking
    {
        let locking_config = locking::time_based(3600);
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Test")),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));

        // 1 millisecond before lock expires - should still be locked
        // 3600 * 1000 - 1 = 3599999 ms
        clock.set_for_testing(initial_time_for_testing() + 3600 * 1000 - 1);
        assert!(trail.is_record_locked(0, &clock), 0);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

// ===== Combined Locking Tests =====

#[test]
fun test_combined_time_and_count_locking_both_lock() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with BOTH time-based (1 hour) and count-based (last 2) locking
    {
        let locking_config = locking::new(
            locking::new_window(std::option::some(3600), std::option::some(2)),
        );
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role and add records
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Add 5 records
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let mut i = 0u64;
        while (i < 5) {
            trail.add_record(
                &record_cap,
                new_test_data(i, b"Record"),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );
            i = i + 1;
        };

        transfer::public_transfer(record_cap, admin);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Test: Records locked by BOTH time and count
    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));

        // Shortly after creation - all records time-locked
        // Records 3, 4 also count-locked (last 2)
        clock.set_for_testing(initial_time_for_testing() + 2000);

        // All records should be locked (time lock active for all)
        assert!(trail.is_record_locked(0, &clock), 0);
        assert!(trail.is_record_locked(1, &clock), 1);
        assert!(trail.is_record_locked(2, &clock), 2);
        assert!(trail.is_record_locked(3, &clock), 3);
        assert!(trail.is_record_locked(4, &clock), 4);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_combined_locking_time_expired_but_count_locked() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with time-based (1 hour) and count-based (last 2) locking
    {
        let locking_config = locking::new(
            locking::new_window(std::option::some(3600), std::option::some(2)),
        );
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role and add records
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Add 5 records
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let mut i = 0u64;
        while (i < 5) {
            trail.add_record(
                &record_cap,
                new_test_data(i, b"Record"),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );
            i = i + 1;
        };

        transfer::public_transfer(record_cap, admin);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Test: Time lock expired, but count lock still active for last 2 records
    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));

        // 2 hours after creation - time lock expired
        clock.set_for_testing(initial_time_for_testing() + 7200 * 1000);

        // Records 0, 1, 2 should be unlocked (time expired, not in last 2)
        assert!(!trail.is_record_locked(0, &clock), 0);
        assert!(!trail.is_record_locked(1, &clock), 1);
        assert!(!trail.is_record_locked(2, &clock), 2);

        // Records 3, 4 should still be locked (count lock - last 2)
        assert!(trail.is_record_locked(3, &clock), 3);
        assert!(trail.is_record_locked(4, &clock), 4);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_combined_locking_count_satisfied_but_time_locked() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with time-based (1 hour) and count-based (last 2) locking
    {
        let locking_config = locking::new(
            locking::new_window(std::option::some(3600), std::option::some(2)),
        );
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role and add records
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Add 5 records
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let mut i = 0u64;
        while (i < 5) {
            trail.add_record(
                &record_cap,
                new_test_data(i, b"Record"),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );
            i = i + 1;
        };

        transfer::public_transfer(record_cap, admin);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    // Test: Count lock satisfied (not in last 2), but time lock still active
    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));

        // Only 30 minutes after creation - time lock still active
        clock.set_for_testing(initial_time_for_testing() + 1800 * 1000);

        // Record 0 is NOT in last 2 (count satisfied), but still time-locked
        // Combined locking uses OR logic: locked if EITHER is true
        assert!(trail.is_record_locked(0, &clock), 0);
        assert!(trail.is_record_locked(1, &clock), 1);
        assert!(trail.is_record_locked(2, &clock), 2);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_combined_locking_both_satisfied_can_delete() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create trail with time-based (1 hour) and count-based (last 2) locking
    {
        let locking_config = locking::new(
            locking::new_window(std::option::some(3600), std::option::some(2)),
        );
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role and add records
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .roles_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.roles_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Add 5 records
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let mut i = 0u64;
        while (i < 5) {
            trail.add_record(
                &record_cap,
                new_test_data(i, b"Record"),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );
            i = i + 1;
        };

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Test: Both locks satisfied - can delete
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));

        // 2 hours after creation - time lock expired
        // Record 0 is not in last 2 - count lock satisfied
        clock.set_for_testing(initial_time_for_testing() + 7200 * 1000);

        // Verify record 0 is unlocked (both conditions satisfied)
        assert!(!trail.is_record_locked(0, &clock), 0);
        assert!(trail.has_record(0), 1);

        // Delete should succeed
        trail.delete_record(&record_cap, 0, &clock, ts::ctx(&mut scenario));

        // Verify deletion
        assert!(!trail.has_record(0), 2);

        clock::destroy_for_testing(clock);
        record_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    ts::end(scenario);
}
