// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient } from "./util";

export async function fetchTrail(): Promise<void> {
    console.log("Fetching an existing audit trail");

    const client = await getFundedClient();
    const { output: createdTrail } = await createTrailWithSeedRecord(client);

    const fetchedTrail = await client.readOnly().trail(createdTrail.id).get();

    console.log("Fetched trail:", fetchedTrail);
    assert.equal(fetchedTrail.id, createdTrail.id);
    assert.equal(fetchedTrail.sequenceNumber, createdTrail.sequenceNumber);
    assert.equal(fetchedTrail.immutableMetadata?.name, createdTrail.immutableMetadata?.name);
}
