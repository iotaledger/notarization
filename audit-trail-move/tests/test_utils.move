#[test_only]
module audit_trail::test_utils;

use audit_trail::{capability::Capability, locking, main::{Self, AuditTrail}};
use iota::{clock::{Self, Clock}, test_scenario::{Self as ts, Scenario}};
use std::string;

const INITIAL_TIME_FOR_TESTING: u64 = 1234;

/// Test data type for audit trail records
public struct TestData has copy, drop, store {
    value: u64,
    message: vector<u8>,
}

public(package) fun new_test_data(value: u64, message: vector<u8>): TestData {
    TestData {
        value,
        message,
    }
}

public(package) fun test_data_value(data: &TestData): u64 {
    data.value
}

public(package) fun test_data_message(data: &TestData): vector<u8> {
    data.message
}

public(package) fun initial_time_for_testing(): u64 {
    INITIAL_TIME_FOR_TESTING
}

/// Setup a test audit trail with optional initial data
public(package) fun setup_test_audit_trail(
    scenario: &mut Scenario,
    locking_config: locking::LockingConfig,
    initial_data: Option<TestData>,
): (Capability, iota::object::ID) {
    let (admin_cap, trail_id) = {
        let mut clock = clock::create_for_testing(ts::ctx(scenario));
        clock.set_for_testing(INITIAL_TIME_FOR_TESTING);

        let trail_metadata = main::new_trail_metadata(
            string::utf8(b"Setup Test Trail"),
            option::some(string::utf8(b"Setup Test Trail Description")),
        );

        let (admin_cap, trail_id) = main::create<TestData>(
            initial_data,
            option::none(),
            locking_config,
            option::some(trail_metadata),
            option::none(),
            &clock,
            ts::ctx(scenario),
        );

        clock::destroy_for_testing(clock);
        (admin_cap, trail_id)
    };

    (admin_cap, trail_id)
}

public(package) fun fetch_capability_trail_and_clock(
    scenario: &mut Scenario,
): (Capability, AuditTrail<TestData>, Clock) {
    let admin_cap = ts::take_from_sender<Capability>(scenario);
    let trail = ts::take_shared<AuditTrail<TestData>>(scenario);
    let clock = iota::clock::create_for_testing(ts::ctx(scenario));
    (admin_cap, trail, clock)
}

public(package) fun cleanup_capability_trail_and_clock(
    scenario: &Scenario,
    cap: Capability,
    trail: AuditTrail<TestData>,
    clock: Clock,
) {
    iota::clock::destroy_for_testing(clock);
    ts::return_to_sender(scenario, cap);
    ts::return_shared(trail);
}

public(package) fun cleanup_trail_and_clock(trail: AuditTrail<TestData>, clock: Clock) {
    iota::clock::destroy_for_testing(clock);
    ts::return_shared(trail);
}
