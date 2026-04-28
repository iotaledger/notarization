// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail, defines the FinanceWriter role restricted to the
 *   `finance` tag, and issues a capability bound to `financeWriterClient`'s address.
 * - **Finance writer client**: Holds the address-bound capability. Can add `finance`-tagged
 *   records but is blocked from writing `legal`-tagged records.
 *
 * Demonstrates how to:
 * 1. Create a trail with a predefined tag registry.
 * 2. Define a role that is restricted to one record tag.
 * 3. Issue a capability bound to a specific wallet address.
 * 4. Show that the holder can add only records matching the allowed tag.
 */

import { CapabilityIssueOptions, Data, Permission, PermissionSet, RoleTags } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "../util";

export async function taggedRecords(): Promise<void> {
    console.log("=== Audit Trail Advanced: Tagged Records ===\n");

    const adminClient = await getFundedClient();
    const financeWriterClient = await getFundedClient();

    const { output: createdTrail } = await adminClient
        .createTrail()
        .withRecordTags(["finance", "legal"])
        .withInitialRecordString("Trail created", "event:created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const trailId = createdTrail.id;

    // The role is scoped to the "finance" tag before the capability is issued.
    const financeWriterRole = adminClient.trail(trailId).access().forRole("FinanceWriter");
    await financeWriterRole
        .create(new PermissionSet([Permission.AddRecord]), new RoleTags(["finance"]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const financeWriterCapability = await financeWriterRole
        .issueCapability(new CapabilityIssueOptions(financeWriterClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    console.log(
        "Issued FinanceWriter capability",
        financeWriterCapability.output.capabilityId,
        "to",
        financeWriterClient.senderAddress(),
        "\n",
    );

    // Capability selection is automatic from financeWriterClient's wallet.
    const financeRecords = financeWriterClient.trail(trailId).records();

    // Add a record with the allowed tag.
    const addedFinanceRecord = await financeRecords
        .add(Data.fromString("Invoice approved"), "department:finance", "finance")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(financeWriterClient);

    console.log(
        "Added tagged record at sequence number",
        addedFinanceRecord.output.sequenceNumber,
        "with tag \"finance\".\n",
    );

    // Attempt to add a record with a different tag — should fail.
    let wrongTagSucceeded = false;
    try {
        await financeRecords
            .add(Data.fromString("Legal review completed"), "department:legal", "legal")
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(financeWriterClient);
        wrongTagSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(wrongTagSucceeded, false, "a finance-scoped role must not add a legal-tagged record");

    const financeRecord = await financeRecords.get(addedFinanceRecord.output.sequenceNumber);
    console.log("Stored tagged record:", financeRecord);
    assert.equal(financeRecord.tag, "finance");
}
