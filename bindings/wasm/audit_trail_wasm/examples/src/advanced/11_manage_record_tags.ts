// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { CapabilityIssueOptions, Data, PermissionSet, RoleTags } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "../util";

/**
 * Demonstrates how to:
 * 1. Delegate record-tag registry management to a TagAdmin role.
 * 2. Add and remove tags from the trail registry.
 * 3. Show that tags still in use by roles or records cannot be removed.
 */
export async function manageRecordTags(): Promise<void> {
    console.log("=== Audit Trail Advanced: Manage Record Tags ===\n");

    const client = await getFundedClient();

    const { output: created } = await client
        .createTrail()
        .withRecordTags(["finance"])
        .withInitialRecordString("Trail created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const trailId = created.id;
    const trailHandle = client.trail(trailId);

    // Delegate tag management to a TagAdmin role
    const tagAdminRole = trailHandle.access().forRole("TagAdmin");
    await tagAdminRole
        .create(PermissionSet.tagAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await tagAdminRole
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // Add a new tag
    await trailHandle.tags().add("legal").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);

    let onChain = await trailHandle.get();
    console.log('Registry after adding "legal":', onChain.tags.map((t) => t.tag), "\n");
    assert.ok(onChain.tags.some((t) => t.tag === "finance"));
    assert.ok(onChain.tags.some((t) => t.tag === "legal"));

    // Create a role scoped to "finance" tag
    await trailHandle
        .access()
        .forRole("FinanceWriter")
        .create(PermissionSet.recordAdminPermissions(), new RoleTags(["finance"]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await trailHandle
        .access()
        .forRole("FinanceWriter")
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // Add a record using the "finance" tag
    await trailHandle
        .records()
        .add(Data.fromString("Tagged finance entry"), undefined, "finance")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // Attempt to remove "finance" tag — should fail because it's in use
    let removeFinanceSucceeded = false;
    try {
        await trailHandle.tags().remove("finance").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);
        removeFinanceSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(removeFinanceSucceeded, false, "a tag referenced by a role or record must not be removable");

    // Remove "legal" tag — should succeed because nothing uses it
    await trailHandle.tags().remove("legal").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);

    onChain = await trailHandle.get();
    console.log('Registry after removing "legal":', onChain.tags.map((t) => t.tag), "\n");
    assert.ok(onChain.tags.some((t) => t.tag === "finance"), "finance tag should still exist");
    assert.ok(!onChain.tags.some((t) => t.tag === "legal"), "legal tag should be removed");

    console.log("Tag management completed successfully.");
}