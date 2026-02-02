#[test_only]
module audit_trail::test_utils;

use audit_trail::{locking, main::{Self, AuditTrail}};
use iota::{clock::{Self, Clock}, test_scenario::{Self as ts, Scenario}};
use std::string;
use tf_components::{capability::Capability, role_map::RoleMap};

const INITIAL_TIME_FOR_TESTING: u64 = 1234567;

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

/// Create a new unrestricted capability with a specific role without any
/// address, valid_from, or valid_until restrictions.
///
/// Returns the newly created capability.
///
/// Sends a CapabilityIssued event upon successful creation.
///
/// Errors:
/// - Aborts with EPermissionDenied if the provided capability does not have the permission specified with `CapabilityAdminPermissions::add`.
/// - Aborts with ERoleDoesNotExist if the specified role does not exist in the role_map.
public fun new_capability_without_restrictions<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: &string::String,
    clock: &Clock,
    ctx: &mut TxContext,
): Capability {
    role_map.new_capability(
        cap,
        role,
        std::option::none(),
        std::option::none(),
        std::option::none(),
        clock,
        ctx,
    )
}

/// Create a new capability with a specific role that expires at a given timestamp (milliseconds since Unix epoch).
///
/// Returns the newly created capability.
///
/// Sends a CapabilityIssued event upon successful creation.
///
/// Errors:
/// - Aborts with EPermissionDenied if the provided capability does not have the permission specified with `CapabilityAdminPermissions::add`.
/// - Aborts with ERoleDoesNotExist if the specified role does not exist in the role_map.
public(package) fun new_capability_valid_until<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: &string::String,
    valid_until: u64,
    clock: &Clock,
    ctx: &mut TxContext,
): Capability {
    role_map.new_capability(
        cap,
        role,
        std::option::none(),
        std::option::none(),
        std::option::some(valid_until),
        clock,
        ctx,
    )
}

/// Create a new capability with a specific role restricted to an address.
/// Optionally set an expiration time (milliseconds since Unix epoch).
///
/// Returns the newly created capability.
///
/// Sends a CapabilityIssued event upon successful creation.
///
/// Errors:
/// - Aborts with EPermissionDenied if the provided capability does not have the permission specified with `CapabilityAdminPermissions::add`.
/// - Aborts with ERoleDoesNotExist if the specified role does not exist in the role_map.
public fun new_capability_for_address<P: copy + drop>(
    role_map: &mut RoleMap<P>,
    cap: &Capability,
    role: &string::String,
    issued_to: address,
    valid_until: Option<u64>,
    clock: &Clock,
    ctx: &mut TxContext,
): Capability {
    role_map.new_capability(
        cap,
        role,
        std::option::some(issued_to),
        std::option::none(),
        valid_until,
        clock,
        ctx,
    )
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
