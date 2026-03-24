#[allow(lint(abort_without_constant))]
#[test_only]
module audit_trail::record_tests;

use audit_trail::{
    locking,
    record::{Self, Data},
    main::{Self, AuditTrail},
    permission,
    record_tags,
    test_utils::{
        Self,
        setup_test_audit_trail,
        setup_test_audit_trail_with_tags,
        initial_time_for_testing,
        fetch_capability_trail_and_clock,
        cleanup_capability_trail_and_clock,
        cleanup_trail_and_clock
    }
};
use iota::{clock, test_scenario as ts};
use std::string;
use tf_components::timelock;

// ===== Add Record Tests =====

#[test]
fun test_add_record_to_empty_trail() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail
    {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
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
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Add record
    ts::next_tx(&mut scenario, admin);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Verify initial state
        assert!(trail.record_count() == 0, 0);
        assert!(trail.is_empty(), 1);

        // Add record
        trail.add_record(
            &record_cap,
            record::new_text(string::utf8(b"First record")),
            std::option::some(string::utf8(b"metadata")),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Verify record was added
        assert!(trail.record_count() == 1, 2);
        assert!(!trail.is_empty(), 3);
        assert!(trail.has_record(0), 4);

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
fun test_add_tagged_record_with_matching_role_tags() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        let (admin_cap, _) = setup_test_audit_trail_with_tags(
            &mut scenario,
            locking_config,
            std::option::none(),
            vector[string::utf8(b"finance")],
        );
        transfer::public_transfer(admin_cap, admin);
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail.create_role(
            &admin_cap,
            string::utf8(b"TaggedWriter"),
            permission::record_admin_permissions(),
            std::option::some(record_tags::new_role_tags(vector[string::utf8(b"finance")])),
            &clock,
            ts::ctx(&mut scenario),
        );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"TaggedWriter"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        trail.add_record(
            &record_cap,
            record::new_text(string::utf8(b"Tagged record")),
            std::option::none(),
            std::option::some(string::utf8(b"finance")),
            &clock,
            ts::ctx(&mut scenario),
        );

        let stored_record = trail.get_record(0);
        assert!(*record::tag(stored_record) == std::option::some(string::utf8(b"finance")), 0);

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = audit_trail::main::ERecordTagNotAllowed)]
fun test_add_tagged_record_requires_matching_role_tags() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        let (admin_cap, _) = setup_test_audit_trail_with_tags(
            &mut scenario,
            locking_config,
            std::option::none(),
            vector[string::utf8(b"finance")],
        );
        transfer::public_transfer(admin_cap, admin);
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"PlainWriter"),
                permission::record_admin_permissions(),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"PlainWriter"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        trail.add_record(
            &record_cap,
            record::new_text(string::utf8(b"Denied tagged record")),
            std::option::none(),
            std::option::some(string::utf8(b"finance")),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = audit_trail::main::ERecordTagNotDefined)]
fun test_add_tagged_record_requires_trail_defined_tag() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        let (admin_cap, _) = setup_test_audit_trail_with_tags(
            &mut scenario,
            locking_config,
            std::option::none(),
            vector[string::utf8(b"legal")],
        );
        transfer::public_transfer(admin_cap, admin);
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail.create_role(
            &admin_cap,
            string::utf8(b"TaggedWriter"),
            permission::record_admin_permissions(),
            std::option::some(record_tags::new_role_tags(vector[string::utf8(b"finance")])),
            &clock,
            ts::ctx(&mut scenario),
        );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"TaggedWriter"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        trail.add_record(
            &record_cap,
            record::new_text(string::utf8(b"Undefined tagged record")),
            std::option::none(),
            std::option::some(string::utf8(b"finance")),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = audit_trail::main::ERecordTagInUse)]
fun test_remove_record_tag_rejects_in_use_tag() {
    let admin = @0xAD;
    let writer = @0xB0B;
    let mut scenario = ts::begin(admin);

    {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        let (admin_cap, _) = setup_test_audit_trail_with_tags(
            &mut scenario,
            locking_config,
            std::option::none(),
            vector[string::utf8(b"finance")],
        );
        transfer::public_transfer(admin_cap, admin);
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail.create_role(
            &admin_cap,
            string::utf8(b"TaggedWriter"),
            permission::record_admin_permissions(),
            std::option::some(record_tags::new_role_tags(vector[string::utf8(b"finance")])),
            &clock,
            ts::ctx(&mut scenario),
        );

        let writer_cap = test_utils::new_capability_for_address(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"TaggedWriter"),
            writer,
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(writer_cap, writer);
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    ts::next_tx(&mut scenario, writer);
    {
        let (writer_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        trail.add_record(
            &writer_cap,
            record::new_text(string::utf8(b"Tagged")),
            std::option::none(),
            std::option::some(string::utf8(b"finance")),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(writer_cap, writer);
        cleanup_trail_and_clock(trail, clock);
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail.remove_record_tag(
            &admin_cap,
            string::utf8(b"finance"),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
fun test_add_multiple_records() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail
    {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
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
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Add multiple records
    ts::next_tx(&mut scenario, admin);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Add 3 records
        let mut i = 0u64;
        while (i < 3) {
            trail.add_record(
                &record_cap,
                record::new_text(string::utf8(b"Record")),
                std::option::none(),
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

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = audit_trail::role_map::ECapabilityPermissionDenied)]
fun test_add_record_permission_denied() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail
    {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
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
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let perms = permission::from_vec(vector[permission::delete_record()]);
        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"NoAddPerm"),
                perms,
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let no_add_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"NoAddPerm"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(no_add_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Try to add record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let (no_add_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // This should fail - no AddRecord permission
        trail.add_record(
            &no_add_cap,
            record::new_text(string::utf8(b"Should fail")),
            std::option::none(),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        cleanup_capability_trail_and_clock(&scenario, no_add_cap, trail, clock);
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
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(record::new_text(string::utf8(b"Initial"))),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Delete record
    ts::next_tx(&mut scenario, admin);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
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

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = audit_trail::role_map::ECapabilityPermissionDenied)]
fun test_delete_record_permission_denied() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Setup trail with initial record
    {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(record::new_text(string::utf8(b"Initial"))),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create role WITHOUT DeleteRecord permission
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        let perms = permission::from_vec(vector[permission::add_record()]);
        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"NoDeletePerm"),
                perms,
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let no_delete_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"NoDeletePerm"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(no_delete_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Try to delete record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let (no_delete_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // This should fail - no DeleteRecord permission
        trail.delete_record(&no_delete_cap, 0, &clock, ts::ctx(&mut scenario));

        cleanup_capability_trail_and_clock(&scenario, no_delete_cap, trail, clock);
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
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
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
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Try to delete non-existent record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // This should fail - record doesn't exist
        trail.delete_record(&record_cap, 999, &clock, ts::ctx(&mut scenario));

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
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
        let locking_config = locking::new(
            locking::window_time_based(3600),
            timelock::none(),
            timelock::none(),
        ); // 1 hour
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(record::new_text(string::utf8(b"Locked record"))),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Try to delete locked record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        // Time is only 1 second after creation - still within lock window
        clock.set_for_testing(initial_time_for_testing() + 1000); // +1 second

        // This should fail - record is time-locked
        trail.delete_record(&record_cap, 0, &clock, ts::ctx(&mut scenario));

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
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
        let locking_config = locking::new(
            locking::window_count_based(5),
            timelock::none(),
            timelock::none(),
        ); // Last 5 records locked
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(record::new_text(string::utf8(b"Locked record"))),
        );
        transfer::public_transfer(admin_cap, admin);
    };

    // Create RecordAdmin role
    ts::next_tx(&mut scenario, admin);
    {
        let (admin_cap, mut trail, clock) = fetch_capability_trail_and_clock(&mut scenario);

        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        transfer::public_transfer(record_cap, admin);
        admin_cap.destroy_for_testing();
        cleanup_trail_and_clock(trail, clock);
    };

    // Try to delete locked record - should fail
    ts::next_tx(&mut scenario, admin);
    {
        let (record_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Only 1 record exists, and last 5 are locked, so it's locked
        trail.delete_record(&record_cap, 0, &clock, ts::ctx(&mut scenario));

        cleanup_capability_trail_and_clock(&scenario, record_cap, trail, clock);
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
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        let initial_data = record::new_bytes(b"Test data");
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(initial_data),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);

        let record = trail.get_record(0);
        let data = audit_trail::record::data(record);

        assert!(record::bytes(data) == option::some(b"Test data"), 0);

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
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);

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
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
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
        let (admin_cap, mut trail, mut clock) = fetch_capability_trail_and_clock(&mut scenario);

        // Empty trail
        assert!(trail.first_sequence().is_none(), 0);
        assert!(trail.last_sequence().is_none(), 1);

        trail
            .access_mut()
            .create_role(
                &admin_cap,
                string::utf8(b"RecordAdmin"),
                permission::record_admin_permissions(),
                std::option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );

        let record_cap = test_utils::new_capability_without_restrictions(
            trail.access_mut(),
            &admin_cap,
            &string::utf8(b"RecordAdmin"),
            &clock,
            ts::ctx(&mut scenario),
        );

        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Add first record
        trail.add_record(
            &record_cap,
            record::new_text(string::utf8(b"First")),
            std::option::none(),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.first_sequence() == std::option::some(0), 2);
        assert!(trail.last_sequence() == std::option::some(0), 3);

        // Add second record
        trail.add_record(
            &record_cap,
            record::new_text(string::utf8(b"Second")),
            std::option::none(),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.first_sequence() == std::option::some(0), 4);
        assert!(trail.last_sequence() == std::option::some(1), 5);

        // Add third record
        trail.add_record(
            &record_cap,
            record::new_text(string::utf8(b"Third")),
            std::option::none(),
            std::option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.first_sequence() == std::option::some(0), 6);
        assert!(trail.last_sequence() == std::option::some(2), 7);

        record_cap.destroy_for_testing();
        cleanup_capability_trail_and_clock(&scenario, admin_cap, trail, clock);
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
        let locking_config = locking::new(
            locking::window_time_based(3600),
            timelock::none(),
            timelock::none(),
        );
        let (admin_cap, _) = setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::none(),
        );
        admin_cap.destroy_for_testing();
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);

        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // This should fail - record doesn't exist
        let _locked = trail.is_record_locked(0, &clock);

        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}
