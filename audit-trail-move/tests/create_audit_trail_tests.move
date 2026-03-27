#[test_only]
module audit_trail::create_audit_trail_tests;

use audit_trail::{
    locking,
    audit_trail::{Self as at, AuditTrail},
    record::{Self, Data},
    test_utils::{
        setup_test_audit_trail,
        initial_time_for_testing,
        create_full_admin_cap,
        destroy_test_cap,
    }
};
use iota::{clock, test_scenario as ts};
use std::string;
use tf_components::timelock;

#[test]
fun test_create_without_initial_record() {
    let user = @0xA;
    let mut scenario = ts::begin(user);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_count_based(0),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(
            &mut scenario,
            locking_config,
            option::none(),
        )
    };

    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);

        assert!(trail.creator() == user);
        assert!(trail.created_at() == initial_time_for_testing());
        assert!(trail.record_count() == 0);
        assert!(trail.id() == trail_id);
        assert!(trail.trusted_source().is_some());

        ts::return_shared(trail);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_create_with_initial_record() {
    let user = @0xB;
    let mut scenario = ts::begin(user);

    let (authority_uid, _trail_id) = {
        let locking_config = locking::new(
            locking::window_time_based(86400),
            timelock::none(),
            timelock::none(),
        );
        let initial_data = record::new_text(string::utf8(b"Hello, World!"));
        setup_test_audit_trail(
            &mut scenario,
            locking_config,
            std::option::some(initial_data),
        )
    };

    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);

        assert!(trail.creator() == user);
        assert!(trail.created_at() == initial_time_for_testing());
        assert!(trail.record_count() == 1);
        assert!(trail.has_record(0));

        ts::return_shared(trail);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_create_minimal_metadata() {
    let user = @0xC;
    let mut scenario = ts::begin(user);

    {
        let mut clock = clock::create_for_testing(ts::ctx(&mut scenario));
        clock.set_for_testing(3000);

        let locking_config = locking::new(
            locking::window_count_based(0),
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

    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);

        assert!(trail.creator() == user);
        assert!(trail.created_at() == 3000);
        assert!(trail.record_count() == 0);
        // trusted_source not set yet
        assert!(trail.trusted_source().is_none());

        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_create_with_locking_enabled() {
    let user = @0xD;
    let mut scenario = ts::begin(user);

    let (authority_uid, _) = {
        let locking_config = locking::new(
            locking::window_time_based(604800),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(
            &mut scenario,
            locking_config,
            option::none(),
        )
    };

    ts::next_tx(&mut scenario, user);
    {
        let trail = ts::take_shared<AuditTrail<Data>>(&scenario);

        assert!(trail.creator() == user);
        assert!(trail.record_count() == 0);

        ts::return_shared(trail);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}

#[test]
fun test_set_trusted_source() {
    let user = @0xA;
    let mut scenario = ts::begin(user);

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

    ts::next_tx(&mut scenario, user);
    {
        let mut trail = ts::take_shared<AuditTrail<Data>>(&scenario);
        assert!(trail.trusted_source().is_none());

        let authority_uid = object::new(ts::ctx(&mut scenario));
        let authority_id = object::uid_to_inner(&authority_uid);

        at::set_trusted_source(&mut trail, authority_id, ts::ctx(&mut scenario));
        assert!(trail.trusted_source().is_some());
        assert!(*trail.trusted_source().borrow() == authority_id);

        object::delete(authority_uid);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
#[expected_failure(abort_code = at::ENotCreator)]
fun test_set_trusted_source_not_creator_aborts() {
    let creator = @0xA;
    let other = @0xB;
    let mut scenario = ts::begin(creator);

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

    // Other user tries to set trusted source
    ts::next_tx(&mut scenario, other);
    {
        let mut trail = ts::take_shared<AuditTrail<Data>>(&scenario);
        let authority_uid = object::new(ts::ctx(&mut scenario));
        let authority_id = object::uid_to_inner(&authority_uid);

        // Should abort — not the creator
        at::set_trusted_source(&mut trail, authority_id, ts::ctx(&mut scenario));

        object::delete(authority_uid);
        ts::return_shared(trail);
    };

    ts::end(scenario);
}

#[test]
fun test_tag_management_with_operation_cap() {
    let admin = @0xA;
    let mut scenario = ts::begin(admin);

    let (authority_uid, trail_id) = {
        let locking_config = locking::new(
            locking::window_none(),
            timelock::none(),
            timelock::none(),
        );
        setup_test_audit_trail(
            &mut scenario,
            locking_config,
            option::none(),
        )
    };

    ts::next_tx(&mut scenario, admin);
    {
        let mut trail = ts::take_shared<AuditTrail<Data>>(&scenario);

        let cap = create_full_admin_cap(&authority_uid, trail_id, admin);

        // Add tag
        trail.add_record_tag(&cap, string::utf8(b"finance"), ts::ctx(&mut scenario));
        let available_tags = trail.tags().tag_keys();
        assert!(available_tags.length() == 1);
        assert!(available_tags.contains(&string::utf8(b"finance")));

        // Remove tag
        trail.remove_record_tag(&cap, string::utf8(b"finance"), ts::ctx(&mut scenario));
        let available_tags = trail.tags().tag_keys();
        assert!(available_tags.length() == 0);

        destroy_test_cap(&authority_uid, cap);
        ts::return_shared(trail);
    };

    object::delete(authority_uid);
    ts::end(scenario);
}
