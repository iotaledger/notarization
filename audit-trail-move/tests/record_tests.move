#[test_only]
module audit_trail::record_tests;

use audit_trail::{
    actions,
    locking,
    audit_trail::{Self as at, AuditTrail},
    record::{Self, Data},
    test_utils::{
        setup_test_audit_trail,
        setup_test_audit_trail_with_tags,
        initial_time_for_testing,
        fetch_trail_and_clock,
        cleanup_trail_and_clock,
        create_record_admin_cap,
        create_full_admin_cap,
        create_test_cap,
        destroy_test_cap,
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

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        assert!(trail.record_count() == 0);
        assert!(trail.is_empty());

        let cap = create_record_admin_cap(&authority_uid, trail_id, admin);

        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"First record")),
            option::some(string::utf8(b"metadata")),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.record_count() == 1);
        assert!(!trail.is_empty());
        assert!(trail.has_record(0));

        let record = trail.get_record(0);
        assert!(record.added_by() == admin);

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_add_multiple_records() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let cap = create_record_admin_cap(&authority_uid, trail_id, admin);

        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"Record 1")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"Record 2")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.record_count() == 2);
        assert!(trail.has_record(0));
        assert!(trail.has_record(1));

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_add_record_with_tag() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail_with_tags(
            &mut scenario,
            locking_config,
            option::none(),
            vector[string::utf8(b"finance")],
        )
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let cap = create_record_admin_cap(&authority_uid, trail_id, admin);

        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"Tagged record")),
            option::none(),
            option::some(string::utf8(b"finance")),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.record_count() == 1);
        let record = trail.get_record(0);
        assert!(record.tag().is_some());

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = at::ERecordTagNotDefined)]
fun test_add_record_with_undefined_tag_aborts() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let cap = create_record_admin_cap(&authority_uid, trail_id, admin);

        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"Tagged record")),
            option::none(),
            option::some(string::utf8(b"nonexistent")),
            &clock,
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

// ===== Delete Record Tests =====

#[test]
fun test_delete_record() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    // Add a record
    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let cap = create_record_admin_cap(&authority_uid, trail_id, admin);

        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"Record to delete")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.record_count() == 1);

        // Delete the record
        trail.delete_record(&cap, 0, &clock, ts::ctx(&mut scenario));
        assert!(trail.record_count() == 0);

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_delete_records_batch() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let cap = create_full_admin_cap(&authority_uid, trail_id, admin);

        // Add 3 records
        let mut i = 0;
        while (i < 3) {
            trail.add_record(
                &cap,
                record::new_text(string::utf8(b"Record")),
                option::none(),
                option::none(),
                &clock,
                ts::ctx(&mut scenario),
            );
            i = i + 1;
        };
        assert!(trail.record_count() == 3);

        // Delete 2 in batch
        let deleted = trail.delete_records_batch(&cap, 2, &clock, ts::ctx(&mut scenario));
        assert!(deleted == 2);
        assert!(trail.record_count() == 1);

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

// ===== Authorization Failure Tests =====

#[test]
#[expected_failure(abort_code = at::ETrustedSourceNotSet)]
fun test_add_record_without_trusted_source_aborts() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    {
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(1000);
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        let _trail_id = at::create<Data>(
            option::none(),
            locking_config,
            option::none(),
            option::none(),
            vector[],
            &clock,
            ts::ctx(&mut scenario),
        );
        clock::destroy_for_testing(clock);
    };

    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<Data>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(2000);

        // Create a cap with arbitrary authority
        let authority_uid = object::new(ts::ctx(&mut scenario));
        let cap = create_record_admin_cap(&authority_uid, trail.id(), admin);

        // Should abort — trusted_source not set
        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"fail")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&authority_uid, cap);
        object::delete(authority_uid);
        clock::destroy_for_testing(clock);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = at::ESourceMismatch)]
fun test_add_record_wrong_source_aborts() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Create cap from a DIFFERENT authority
        let wrong_uid = object::new(ts::ctx(&mut scenario));
        let cap = create_record_admin_cap(&wrong_uid, trail_id, admin);

        // Should abort — source mismatch
        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"fail")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&wrong_uid, cap);
        object::delete(wrong_uid);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = at::EPermissionDenied)]
fun test_add_record_missing_permission_aborts() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Create cap with only DELETE permission, no ADD
        let cap = create_test_cap(
            &authority_uid,
            trail_id,
            vector[actions::delete_record()],
            admin,
        );

        // Should abort — no add_record permission
        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"fail")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = at::EHolderMismatch)]
fun test_add_record_wrong_holder_aborts() {
    let admin = @0xAD;
    let other = @0xBE;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    // Use different sender than cap holder
    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Cap issued to 'other' but tx sender is 'admin'
        let cap = create_record_admin_cap(&authority_uid, trail_id, other);

        // Should abort — holder mismatch
        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"fail")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = at::ETargetMismatch)]
fun test_add_record_wrong_target_aborts() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, _trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        // Create cap targeting a DIFFERENT trail ID
        let fake_target = object::new(ts::ctx(&mut scenario));
        let fake_id = object::uid_to_inner(&fake_target);
        let cap = create_record_admin_cap(&authority_uid, fake_id, admin);

        // Should abort — target mismatch
        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"fail")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&authority_uid, cap);
        object::delete(fake_target);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

// ===== Delete Trail Tests =====

#[test]
fun test_delete_empty_trail() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(&mut scenario, locking_config, option::none())
    };

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);
        let clock = clock::create_for_testing(ts::ctx(&mut scenario));

        let cap = create_test_cap(
            &authority_uid,
            trail_id,
            vector[actions::delete_audit_trail()],
            admin,
        );

        trail.delete_audit_trail(&cap, &clock, ts::ctx(&mut scenario));

        destroy_test_cap(&authority_uid, cap);
        clock::destroy_for_testing(clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}
