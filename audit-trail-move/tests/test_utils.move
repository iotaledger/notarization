#[test_only]
module audit_trail::test_utils;

use audit_trail::{actions, locking, audit_trail::{Self as at, AuditTrail}, marker::AuditTrailPerm, record::{Self, Data}};
use iota::{clock::{Self, Clock}, test_scenario::{Self as ts, Scenario}, vec_set};
use std::string;
use tf_components::operation_cap::{Self, OperationCap};

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

/// Setup a test audit trail with optional initial data.
/// Returns the trail ID and the authority UID used to issue OperationCaps.
public(package) fun setup_test_audit_trail(
    scenario: &mut Scenario,
    locking_config: locking::LockingConfig,
    initial_data: Option<record::Data>,
): (UID, iota::object::ID) {
    setup_test_audit_trail_impl(scenario, locking_config, initial_data, vector[])
}

/// Setup a test audit trail with optional initial data and available record tags.
public(package) fun setup_test_audit_trail_with_tags(
    scenario: &mut Scenario,
    locking_config: locking::LockingConfig,
    initial_data: Option<record::Data>,
    available_record_tags: vector<string::String>,
): (UID, iota::object::ID) {
    setup_test_audit_trail_impl(scenario, locking_config, initial_data, available_record_tags)
}

/// Setup a test audit trail backed by the `TestData` helper type.
public(package) fun setup_test_data_audit_trail(
    scenario: &mut Scenario,
    locking_config: locking::LockingConfig,
    initial_data: Option<TestData>,
): (UID, iota::object::ID) {
    setup_test_audit_trail_impl(scenario, locking_config, initial_data, vector[])
}

fun setup_test_audit_trail_impl<D: store + copy + drop>(
    scenario: &mut Scenario,
    locking_config: locking::LockingConfig,
    initial_data: Option<D>,
    available_record_tags: vector<string::String>,
): (UID, iota::object::ID) {
    let trail_id = {
        let mut clock = clock::create_for_testing(ts::ctx(scenario));
        clock.set_for_testing(INITIAL_TIME_FOR_TESTING);

        let trail_metadata = at::new_trail_metadata(
            string::utf8(b"Setup Test Trail"),
            option::some(string::utf8(b"Setup Test Trail Description")),
        );

        let initial_record = if (initial_data.is_some()) {
            option::some(
                record::new_initial_record(
                    initial_data.destroy_some(),
                    option::none(),
                    option::none(),
                ),
            )
        } else {
            initial_data.destroy_none();
            option::none()
        };

        let trail_id = at::create<D>(
            initial_record,
            locking_config,
            option::some(trail_metadata),
            option::none(),
            available_record_tags,
            &clock,
            ts::ctx(scenario),
        );

        clock::destroy_for_testing(clock);
        trail_id
    };

    // Create authority UID and bind trail to it
    let authority_uid = object::new(ts::ctx(scenario));
    let authority_id = object::uid_to_inner(&authority_uid);

    // Bind trail to authority in next tx
    let sender = ts::ctx(scenario).sender();
    ts::next_tx(scenario, sender);
    {
        let mut trail = ts::take_shared<AuditTrail<D>>(scenario);
        at::set_trusted_source(&mut trail, authority_id, ts::ctx(scenario));
        ts::return_shared(trail);
    };

    (authority_uid, trail_id)
}

/// Create an OperationCap for testing with the given permissions.
public(package) fun create_test_cap(
    authority_uid: &UID,
    trail_id: ID,
    permissions: vector<u16>,
    holder: address,
): OperationCap<AuditTrailPerm> {
    let perms = vec_set::from_keys(permissions);
    operation_cap::new<AuditTrailPerm>(authority_uid, trail_id, perms, holder)
}

/// Create an OperationCap with all record-related permissions.
public(package) fun create_record_admin_cap(
    authority_uid: &UID,
    trail_id: ID,
    holder: address,
): OperationCap<AuditTrailPerm> {
    create_test_cap(
        authority_uid,
        trail_id,
        vector[
            actions::add_record(),
            actions::correct_record(),
            actions::delete_record(),
            actions::delete_all_records(),
        ],
        holder,
    )
}

/// Create an OperationCap with all permissions (admin-like).
public(package) fun create_full_admin_cap(
    authority_uid: &UID,
    trail_id: ID,
    holder: address,
): OperationCap<AuditTrailPerm> {
    create_test_cap(
        authority_uid,
        trail_id,
        vector[
            actions::add_record(),
            actions::correct_record(),
            actions::delete_record(),
            actions::delete_all_records(),
            actions::update_metadata(),
            actions::delete_metadata(),
            actions::update_locking_config(),
            actions::update_locking_config_for_delete_record(),
            actions::update_locking_config_for_delete_trail(),
            actions::update_locking_config_for_write(),
            actions::add_record_tags(),
            actions::delete_record_tags(),
            actions::delete_audit_trail(),
            actions::migrate(),
        ],
        holder,
    )
}

/// Destroy an OperationCap for testing cleanup.
public(package) fun destroy_test_cap(
    authority_uid: &UID,
    cap: OperationCap<AuditTrailPerm>,
) {
    operation_cap::destroy(authority_uid, cap);
}

/// Fetch trail and clock for a test transaction.
public(package) fun fetch_trail_and_clock(
    scenario: &mut Scenario,
): (AuditTrail<Data>, Clock) {
    let trail = ts::take_shared<AuditTrail<Data>>(scenario);
    let clock = iota::clock::create_for_testing(ts::ctx(scenario));
    (trail, clock)
}

/// Cleanup trail and clock after test.
public(package) fun cleanup_trail_and_clock(trail: AuditTrail<Data>, clock: Clock) {
    iota::clock::destroy_for_testing(clock);
    ts::return_shared(trail);
}
