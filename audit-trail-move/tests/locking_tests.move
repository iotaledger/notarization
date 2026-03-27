#[test_only]
module audit_trail::locking_tests;

use audit_trail::{
    actions,
    locking,
    audit_trail::{Self as at, AuditTrail},
    record::{Self, Data},
    test_utils::{
        setup_test_audit_trail,
        initial_time_for_testing,
        fetch_trail_and_clock,
        cleanup_trail_and_clock,
        create_test_cap,
        create_full_admin_cap,
        create_record_admin_cap,
        destroy_test_cap,
    }
};
use iota::{clock, test_scenario as ts};
use std::string;
use tf_components::timelock;

/// Helper: create a trail with a custom locking config that needs a clock for timelocks,
/// then bind it to an authority. Returns (authority_uid, trail_id).
fun setup_timelocked_trail(
    scenario: &mut ts::Scenario,
    locking_config: locking::LockingConfig,
): (UID, ID) {
    let create_clock = clock::create_for_testing(ts::ctx(scenario));
    let trail_id = at::create<Data>(
        option::none(),
        locking_config,
        option::some(at::new_trail_metadata(
            string::utf8(b"Test Trail"),
            option::none(),
        )),
        option::none(),
        vector[],
        &create_clock,
        ts::ctx(scenario),
    );
    clock::destroy_for_testing(create_clock);

    let authority_uid = object::new(ts::ctx(scenario));
    let authority_id = object::uid_to_inner(&authority_uid);

    let sender = ts::ctx(scenario).sender();
    ts::next_tx(scenario, sender);
    {
        let mut trail = ts::take_shared<AuditTrail<Data>>(scenario);
        at::set_trusted_source(&mut trail, authority_id, ts::ctx(scenario));
        ts::return_shared(trail);
    };

    (authority_uid, trail_id)
}

// ===== Write Lock Tests =====

#[test]
#[expected_failure(abort_code = at::ETrailWriteLocked)]
fun test_add_record_while_write_locked() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    // Create clock first so we can build a timelock
    let mut setup_clock = clock::create_for_testing(ts::ctx(&mut scenario));
    setup_clock.set_for_testing(initial_time_for_testing());

    let locking_config = locking::new(
        locking::window_none(),
        timelock::none(),
        timelock::unlock_at_ms(initial_time_for_testing() + 100_000, &setup_clock),
    );
    clock::destroy_for_testing(setup_clock);

    let (authority_uid, trail_id) = setup_timelocked_trail(&mut scenario, locking_config);

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let cap = create_record_admin_cap(&authority_uid, trail_id, admin);

        // Should abort — write locked
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
fun test_add_record_after_write_lock_expired() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let mut setup_clock = clock::create_for_testing(ts::ctx(&mut scenario));
    setup_clock.set_for_testing(initial_time_for_testing());

    let locking_config = locking::new(
        locking::window_none(),
        timelock::none(),
        timelock::unlock_at_ms(initial_time_for_testing() + 1000, &setup_clock),
    );
    clock::destroy_for_testing(setup_clock);

    let (authority_uid, trail_id) = setup_timelocked_trail(&mut scenario, locking_config);

    ts::next_tx(&mut scenario, admin);
    {
        let (mut trail, mut clock) = fetch_trail_and_clock(&mut scenario);
        clock.set_for_testing(initial_time_for_testing() + 2000);

        let cap = create_record_admin_cap(&authority_uid, trail_id, admin);

        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"ok")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        assert!(trail.record_count() == 1);

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

// ===== Delete Record Lock Tests =====

#[test]
#[expected_failure(abort_code = at::ERecordLocked)]
fun test_delete_record_while_time_locked() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        // 100 second record deletion lock (window-based, no clock needed)
        let locking_config = locking::new(
            locking::window_time_based(100),
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

        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"Record")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Try to delete within 100-second window (only 0.5s later)
        clock.set_for_testing(initial_time_for_testing() + 1500);

        // Should abort — record is time-locked
        trail.delete_record(&cap, 0, &clock, ts::ctx(&mut scenario));

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_delete_record_after_time_lock_expired() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        // 1 second record deletion lock
        let locking_config = locking::new(
            locking::window_time_based(1),
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

        trail.add_record(
            &cap,
            record::new_text(string::utf8(b"Record")),
            option::none(),
            option::none(),
            &clock,
            ts::ctx(&mut scenario),
        );

        // Advance past the 1-second lock
        clock.set_for_testing(initial_time_for_testing() + 3000);

        trail.delete_record(&cap, 0, &clock, ts::ctx(&mut scenario));
        assert!(trail.record_count() == 0);

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

// ===== Delete Trail Lock Tests =====

#[test]
#[expected_failure(abort_code = at::ETrailDeleteLocked)]
fun test_delete_trail_while_locked() {
    let admin = @0xAD;
    let mut scenario = ts::begin(admin);

    let mut setup_clock = clock::create_for_testing(ts::ctx(&mut scenario));
    setup_clock.set_for_testing(initial_time_for_testing());

    let locking_config = locking::new(
        locking::window_none(),
        timelock::unlock_at_ms(initial_time_for_testing() + 100_000, &setup_clock),
        timelock::none(),
    );
    clock::destroy_for_testing(setup_clock);

    let (authority_uid, trail_id) = setup_timelocked_trail(&mut scenario, locking_config);

    ts::next_tx(&mut scenario, admin);
    {
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(initial_time_for_testing() + 1000);

        let cap = create_test_cap(
            &authority_uid,
            trail_id,
            vector[actions::delete_audit_trail()],
            admin,
        );

        // Should abort — delete trail locked
        trail.delete_audit_trail(&cap, &clock, ts::ctx(&mut scenario));

        destroy_test_cap(&authority_uid, cap);
        clock::destroy_for_testing(clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

// ===== Update Locking Config Tests =====

#[test]
fun test_update_locking_config() {
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
        let (mut trail, clock) = fetch_trail_and_clock(&mut scenario);

        let cap = create_test_cap(
            &authority_uid,
            trail_id,
            vector[actions::update_locking_config()],
            admin,
        );

        let new_config = locking::new(
            locking::window_time_based(3600),
            timelock::none(),
            timelock::none(),
        );

        trail.update_locking_config(&cap, new_config, ts::ctx(&mut scenario));

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_update_delete_record_window() {
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
        let (mut trail, clock) = fetch_trail_and_clock(&mut scenario);

        let cap = create_test_cap(
            &authority_uid,
            trail_id,
            vector[actions::update_locking_config_for_delete_record()],
            admin,
        );

        trail.update_delete_record_window(
            &cap,
            locking::window_count_based(5),
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_update_delete_trail_lock() {
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
        clock.set_for_testing(1000);

        let cap = create_test_cap(
            &authority_uid,
            trail_id,
            vector[actions::update_locking_config_for_delete_trail()],
            admin,
        );

        trail.update_delete_trail_lock(
            &cap,
            timelock::unlock_at_ms(99999999, &clock),
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_update_write_lock() {
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
        clock.set_for_testing(1000);

        let cap = create_test_cap(
            &authority_uid,
            trail_id,
            vector[actions::update_locking_config_for_write()],
            admin,
        );

        trail.update_write_lock(
            &cap,
            timelock::unlock_at_ms(99999999, &clock),
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}
