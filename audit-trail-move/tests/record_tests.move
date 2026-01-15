#[test_only]
module audit_trail::record_tests;

use audit_trail::{
    capability::Capability,
    locking,
    main::{Self, AuditTrail},
    permission,
    test_utils::{
        TestData,
        setup_test_audit_trail,
        new_test_data,
        initial_time_for_testing,
        test_data_value,
        test_data_message
    }
};
use iota::{clock, test_scenario as ts};
use std::string;

// ===== Add Record Tests =====

#[test]
fun test_add_record_to_empty_trail() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role
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

    // Add record
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Verify initial state
        assert!(trail.record_count() == 0, 0);
        assert!(trail.is_empty(), 1);

        // Add record
        trail.add_record(
            &record_cap,
            new_test_data(42, b"First record"),
            std::option::some(string::utf8(b"metadata")),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Verify record was added
        assert!(trail.record_count() == 1, 2);
        assert!(!trail.is_empty(), 3);
        assert!(trail.has_record(0), 4);

        clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, record_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_add_multiple_records() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role
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

    // Add multiple records
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Add 3 records
        let mut i = 0u64;
        while (i < 3) {
            trail.add_record(
                &record_cap,
                new_test_data(i, b"Record"),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );
            i = i + 1;
        };

        // Verify all records exist
        assert!(trail.record_count() == 3, 0);
        assert!(trail.has_record(0), 1);
        assert!(trail.has_record(1), 2);
        assert!(trail.has_record(2), 3);
        assert!(!trail.has_record(3), 4);

        clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, record_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
fun test_add_record_permission_denied() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create role WITHOUT AddRecord permission
    ts::next_tx(&mut scenario, admin);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let perms = permission::from_vec(vector[permission::delete_record()]);
        trail.create_role(&admin_cap, string::utf8(b"NoAddPerm"), perms, ts::ctx(&mut scenario));

        let no_add_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NoAddPerm"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(no_add_cap, admin);
        admin_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    // Try to add record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let no_add_cap = ts::take_from_sender<Capability>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // This should fail - no AddRecord permission
        trail.add_record(
            &no_add_cap,
            new_test_data(1, b"Should fail"),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, no_add_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

// ===== Delete Record Tests =====

#[test]
fun test_delete_record_success() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail with initial record
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Initial")),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role
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

    // Delete record
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Verify initial state
        assert!(trail.record_count() == 1, 0);
        assert!(trail.has_record(0), 1);

        // Delete record
        trail.delete_record(&record_cap, 0, &clock, ts::ctx(&mut scenario));

        // Verify record was deleted
        assert!(trail.record_count() == 0, 2); // actual count decreases
        assert!(trail.sequence_number() == 1, 3); // sequence stays monotonic
        assert!(!trail.has_record(0), 4); // record is gone

        clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, record_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::EPermissionDenied)]
fun test_delete_record_permission_denied() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail with initial record
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(new_test_data(1, b"Initial")),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create role WITHOUT DeleteRecord permission
    ts::next_tx(&mut scenario, admin);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let perms = permission::from_vec(vector[permission::add_record()]);
        trail.create_role(&admin_cap, string::utf8(b"NoDeletePerm"), perms, ts::ctx(&mut scenario));

        let no_delete_cap = trail.new_capability(
            &admin_cap,
            &string::utf8(b"NoDeletePerm"),
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(no_delete_cap, admin);
        admin_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    // Try to delete record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let no_delete_cap = ts::take_from_sender<Capability>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // This should fail - no DeleteRecord permission
        trail.delete_record(&no_delete_cap, 0, &clock, ts::ctx(&mut scenario));

        clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, no_delete_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::ERecordNotFound)]
fun test_delete_record_not_found() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail (no initial record)
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role
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

    // Try to delete non-existent record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // This should fail - record doesn't exist
        trail.delete_record(&record_cap, 999, &clock, ts::ctx(&mut scenario));

        clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, record_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::ERecordLocked)]
fun test_delete_record_time_locked() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail with time-based locking and initial record
    {
        let locking_config = locking::time_based(3600); // 1 hour
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

    // Try to delete locked record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        // Time is only 1 second after creation - still within lock window
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000); // +1 second

        // This should fail - record is time-locked
        trail.delete_record(&record_cap, 0, &clock, ts::ctx(&mut scenario));

        clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, record_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::ERecordLocked)]
fun test_delete_record_count_locked() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail with count-based locking and initial record
    {
        let locking_config = locking::count_based(5); // Last 5 records locked
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

    // Try to delete locked record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);
        let record_cap = ts::take_from_sender<Capability>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Only 1 record exists, and last 5 are locked, so it's locked
        trail.delete_record(&record_cap, 0, &clock, ts::ctx(&mut scenario));

        clock::destroy_for_testing(clock);
        ts::return_to_sender(&scenario, record_cap);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

// ===== Query Function Tests =====

#[test]
fun test_get_record() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail with initial record
    {
        let locking_config = locking::none();
        let initial_data = new_test_data(42, b"Test data");
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(initial_data),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let record = trail.get_record(0);
        let data = audit_trail::record::data(record);

        assert!(data.test_data_value() == 42, 0);
        assert!(data.test_data_message() == b"Test data", 1);

        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::ERecordNotFound)]
fun test_get_record_not_found() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail (no initial record)
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // This should fail - no records exist
        let _record = trail.get_record(0);

        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_first_last_sequence() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail
    {
        let locking_config = locking::none();
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin and test sequence functions
    ts::next_tx(&mut scenario, admin);
    {
        let admin_cap = ts::take_from_sender<Capability>(&scenario);
        let mut trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        // Empty trail
        assert!(trail.first_sequence().is_none(), 0);
        assert!(trail.last_sequence().is_none(), 1);

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

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Add first record
        trail.add_record(
            &record_cap,
            new_test_data(1, b"First"),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.first_sequence() == std::option::some(0), 2);
        assert!(trail.last_sequence() == std::option::some(0), 3);

        // Add second record
        trail.add_record(
            &record_cap,
            new_test_data(2, b"Second"),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.first_sequence() == std::option::some(0), 4);
        assert!(trail.last_sequence() == std::option::some(1), 5);

        // Add third record
        trail.add_record(
            &record_cap,
            new_test_data(3, b"Third"),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.first_sequence() == std::option::some(0), 6);
        assert!(trail.last_sequence() == std::option::some(2), 7);

        clock::destroy_for_testing(clock);
        admin_cap.destroy_for_testing();
        record_cap.destroy_for_testing();
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = main::ERecordNotFound)]
fun test_is_record_locked_not_found() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail (no initial record)
    {
        let locking_config = locking::time_based(3600);
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<TestData>>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // This should fail - record doesn't exist
        let _locked = trail.is_record_locked(0, &clock);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}
