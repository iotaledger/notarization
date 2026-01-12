#[test_only]
module audit_trail::locking_tests;

use audit_trail::{
    capability::Capability,
    locking,
    main::{Self, AuditTrail},
    permission,
    test_utils::{TestData, setup_test_audit_trail, new_test_data, initial_time_for_testing}
};
use iota::{clock, test_scenario as ts};
use std::string;

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
        let (admin_cap, _) = setup_test_audit_trail(&mut scenario, locking_config, std::option::none());
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role and capability
    ts::next_tx(&mut scenario, admin);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        trail.create_role(
            &admin_cap,
            string::utf8(b"RecordAdmin"),
            permission::record_admin_permissions(),
            ts::ctx(&mut scenario),
        );

        let record_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        ts::return_shared(trail);
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
            trail.add_record(&record_cap, new_test_data(i, b"Record"), std::option::none(), &clock, ts::ctx(&mut scenario));
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
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let perms = permission::from_vec(vector[permission::update_locking_config()]);
        trail.create_role(&admin_cap, string::utf8(b"LockingAdmin"), perms, ts::ctx(&mut scenario));

        let locking_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"LockingAdmin"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(locking_cap, admin);
        admin_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    // Update from no-locking to time-based
    ts::next_tx(&mut scenario, admin);
    {
        let locking_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Initially unlocked
        assert!(!trail.is_record_locked(0, &clock), 0);

        // Update to 1 hour time-based locking
        trail.update_locking_config(&locking_cap, locking::time_based(3600), ts::ctx(&mut scenario));

        // Now locked
        assert!(trail.is_record_locked(0, &clock), 1);

        clock::destroy_for_testing(clock);
        locking_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
fun test_update_locking_config_permission_denied() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(&mut scenario, locking_config, std::option::none());
        transfer::public_transfer(admin_cap, admin);
    };

    // Create role WITHOUT UpdateLockingConfig permission
    ts::next_tx(&mut scenario, admin);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(&admin_cap, string::utf8(b"NoLockingPerm"), perms, ts::ctx(&mut scenario));

        let no_locking_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NoLockingPerm"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(no_locking_cap, admin);
        admin_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    // Try to update locking config - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let no_locking_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        trail.update_locking_config(&no_locking_cap, locking::time_based(3600), ts::ctx(&mut scenario));

        no_locking_cap.destroy_for_testing();
        ts::return_shared(trail);
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
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let perms = permission::from_vec(vector[permission::update_locking_config_for_delete_record()]);
        trail.create_role(&admin_cap, string::utf8(b"DeleteLockAdmin"), perms, ts::ctx(&mut scenario));

        let delete_lock_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"DeleteLockAdmin"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(delete_lock_cap, admin);
        admin_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    // Update delete_record_lock
    ts::next_tx(&mut scenario, admin);
    {
        let delete_lock_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Initially unlocked
        assert!(!trail.is_record_locked(0, &clock), 0);

        // Update to count-based (last 5 locked)
        trail.update_locking_config_for_delete_record(&delete_lock_cap, locking::window_count_based(5), ts::ctx(&mut scenario));

        // Now locked (single record, last 5 are locked)
        assert!(trail.is_record_locked(0, &clock), 1);

        clock::destroy_for_testing(clock);
        delete_lock_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
fun test_update_locking_config_for_delete_record_permission_denied() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(&mut scenario, locking_config, std::option::none());
        transfer::public_transfer(admin_cap, admin);
    };

    // Create role with update_locking_config but NOT update_locking_config_for_delete_record
    ts::next_tx(&mut scenario, admin);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let perms = permission::from_vec(vector[permission::update_locking_config()]);
        trail.create_role(&admin_cap, string::utf8(b"WrongPerm"), perms, ts::ctx(&mut scenario));

        let wrong_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"WrongPerm"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(wrong_cap, admin);
        admin_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    // Try to update delete_record_lock - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let wrong_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        trail.update_locking_config_for_delete_record(&wrong_cap, locking::window_count_based(5), ts::ctx(&mut scenario));

        wrong_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    ts::end(scenario);
}
