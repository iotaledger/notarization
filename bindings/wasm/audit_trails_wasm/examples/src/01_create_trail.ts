// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient } from "./util";

export async function createTrail(): Promise<void> {
    console.log("Creating an audit trail");

    const client = await getFundedClient();
    const { output: trail, response } = await createTrailWithSeedRecord(client);

    console.log(`Created trail ${trail.id} with transaction ${response.digest}`);
    console.log("Immutable metadata:", trail.immutableMetadata);
    console.log("Updatable metadata:", trail.updatableMetadata);
    console.log("Locking config:", trail.lockingConfig);

    assert.equal(trail.sequenceNumber, 1n);
    assert.ok(trail.immutableMetadata);
    assert.equal(trail.immutableMetadata?.name, "Example Audit Trail");
}
