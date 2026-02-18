// Copyright (c) 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// This module provides a test function using an iota_notarization::Notarization to test compatibility between Move code
/// used as a dependency in the hello_notarization module and the notarization package already published to testnet, mainnet etc.
#[allow(lint(self_transfer))]
module hello_notarization::hello;

use iota::clock::Clock;
use std::string;

use iota_notarization::notarization;
use iota_notarization::{dynamic_notarization, timelock};

public fun create_new_dynamic_notarization(clock: &Clock, ctx: &mut TxContext): notarization::Notarization<vector<u8>> {
    let mut some_data = vector::empty();
    some_data.push_back(1u8);
    some_data.push_back(2u8);

    let state = notarization::new_state_from_generic(some_data, std::option::none());

    dynamic_notarization::new(
        state,
        std::option::some(string::utf8(b"Hello Notarization")),
        std::option::some(string::utf8(b"Updateable Hello Notarization Metadata")),
        timelock::none(),
        clock,
        ctx,
    )
}


public fun destroy_dynamic_notarization(notarization: notarization::Notarization<vector<u8>>, clock: &Clock) {
    notarization::destroy(notarization, clock);
}