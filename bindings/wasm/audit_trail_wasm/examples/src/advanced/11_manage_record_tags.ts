// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates the trail and manages roles.
 * - **Tag admin client**: Holds the TagAdmin capability. Adds and removes entries from the trail's
 *   tag registry.
 * - **Finance writer client**: Holds a `finance`-scoped RecordAdmin capability. Writes a
 *   `finance`-tagged record that keeps the `finance` tag in use and therefore unremovable.
 *
 * Demonstrates how to:
 * 1. Delegate record-tag registry management to a TagAdmin role.
 * 2. Add and remove tags from the trail registry.
 * 3. Show that tags still in use by roles or records cannot be removed.
 */

import { CapabilityIssueOptions, Data, PermissionSet, RoleTags } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "../util";

export async function manageRecordTags(): Promise<void> {
    console.log("=== Audit Trail Advanced: Manage Record Tags ===\n");

    // `adminClient` creates the trail and manages roles.
    // `tagAdminClient` manages tags; `financeWriterClient` writes tagged records.
    const adminClient = await getFundedClient();
    const tagAdminClient = await getFundedClient();
    const financeWriterClient = await getFundedClient();

    const { output: createdTrail } = await adminClient
        .createTrail()
        .withRecordTags(["finance"])
        .withInitialRecordString("Trail created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    const trailId = createdTrail.id;

    // Delegate tag management to a TagAdmin role.
    const tagAdminRole = adminClient.trail(trailId).access().forRole("TagAdmin");
    await tagAdminRole
        .create(PermissionSet.tagAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await tagAdminRole
        .issueCapability(new CapabilityIssueOptions(tagAdminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    // TagAdmin adds a new tag to the registry before any role or record uses it.
    await tagAdminClient.trail(trailId).tags().add("legal").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(
        tagAdminClient,
    );

    let onChainTrail = await adminClient.trail(trailId).get();
    console.log("Registry after adding \"legal\":", onChainTrail.tags.map((t) => t.tag), "\n");
    assert.ok(onChainTrail.tags.some((t) => t.tag === "finance"));
    assert.ok(onChainTrail.tags.some((t) => t.tag === "legal"));

    // Create a role scoped to the "finance" tag and issue to financeWriterClient.
    await adminClient
        .trail(trailId)
        .access()
        .forRole("FinanceWriter")
        .create(PermissionSet.recordAdminPermissions(), new RoleTags(["finance"]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await adminClient
        .trail(trailId)
        .access()
        .forRole("FinanceWriter")
        .issueCapability(new CapabilityIssueOptions(financeWriterClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    // FinanceWriter adds a record using the "finance" tag.
    await financeWriterClient
        .trail(trailId)
        .records()
        .add(Data.fromString("Tagged finance entry"), undefined, "finance")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(financeWriterClient);

    // TagAdmin attempts to remove "finance" tag — should fail because it's in use.
    let removeFinanceSucceeded = false;
    try {
        await tagAdminClient.trail(trailId).tags().remove("finance").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(
            tagAdminClient,
        );
        removeFinanceSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(removeFinanceSucceeded, false, "a tag referenced by a role or record must not be removable");

    // TagAdmin removes "legal" tag — should succeed because nothing uses it.
    await tagAdminClient.trail(trailId).tags().remove("legal").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(
        tagAdminClient,
    );

    onChainTrail = await adminClient.trail(trailId).get();
    console.log("Registry after removing \"legal\":", onChainTrail.tags.map((t) => t.tag), "\n");
    assert.ok(onChainTrail.tags.some((t) => t.tag === "finance"), "finance tag should still exist");
    assert.ok(!onChainTrail.tags.some((t) => t.tag === "legal"), "legal tag should be removed");

    console.log("Tag management completed successfully.");
}
