#[test_only]
module audit_trail::metadata_tests;

use audit_trail::{
    actions,
    locking,
    audit_trail::{Self as at, AuditTrail},
    record::Data,
    test_utils::{
        setup_test_audit_trail,
        fetch_trail_and_clock,
        cleanup_trail_and_clock,
        create_test_cap,
        destroy_test_cap,
    }
};
use iota::test_scenario as ts;
use std::string;
use tf_components::timelock;

#[test]
fun test_update_metadata() {
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
            vector[actions::update_metadata()],
            admin,
        );

        assert!(trail.metadata().is_none());

        trail.update_metadata(
            &cap,
            option::some(string::utf8(b"new metadata")),
            ts::ctx(&mut scenario),
        );

        assert!(trail.metadata().is_some());

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_trail_name_and_description() {
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
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);

        assert!(trail.name().is_some());
        assert!(trail.description().is_some());

        ts::return_shared(trail);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = at::EPermissionDenied)]
fun test_update_metadata_without_permission_aborts() {
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
            vector[actions::add_record()],
            admin,
        );

        trail.update_metadata(
            &cap,
            option::some(string::utf8(b"fail")),
            ts::ctx(&mut scenario),
        );

        destroy_test_cap(&authority_uid, cap);
        cleanup_trail_and_clock(trail, clock);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}
