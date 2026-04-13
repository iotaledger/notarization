// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin**: Creates the trail and manages roles.
 * - **TagAdmin**: Holds the TagAdmin capability. Adds and removes entries from the trail's
 *   tag registry.
 * - **FinanceWriter**: Holds a `finance`-scoped RecordAdmin capability. Writes a
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

    // `admin` creates the trail and manages roles.
    // `tagAdmin` adds/removes tags; `financeWriter` writes tagged records.
    const admin = await getFundedClient();
    const tagAdmin = await getFundedClient();
    const financeWriter = await getFundedClient();

    const { output: created } = await admin
        .createTrail()
        .withRecordTags(["finance"])
        .withInitialRecordString("Trail created")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    const trailId = created.id;

    // Delegate tag management to a TagAdmin role.
    const tagAdminRole = admin.trail(trailId).access().forRole("TagAdmin");
    await tagAdminRole
        .create(PermissionSet.tagAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await tagAdminRole
        .issueCapability(new CapabilityIssueOptions(tagAdmin.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    // TagAdmin adds a new tag.
    await tagAdmin.trail(trailId).tags().add("legal").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(tagAdmin);

    let onChain = await admin.trail(trailId).get();
    console.log("Registry after adding \"legal\":", onChain.tags.map((t) => t.tag), "\n");
    assert.ok(onChain.tags.some((t) => t.tag === "finance"));
    assert.ok(onChain.tags.some((t) => t.tag === "legal"));

    // Create a role scoped to "finance" tag and issue to financeWriter.
    await admin
        .trail(trailId)
        .access()
        .forRole("FinanceWriter")
        .create(PermissionSet.recordAdminPermissions(), new RoleTags(["finance"]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await admin
        .trail(trailId)
        .access()
        .forRole("FinanceWriter")
        .issueCapability(new CapabilityIssueOptions(financeWriter.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    // FinanceWriter adds a record using the "finance" tag.
    await financeWriter
        .trail(trailId)
        .records()
        .add(Data.fromString("Tagged finance entry"), undefined, "finance")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(financeWriter);

    // TagAdmin attempts to remove "finance" tag — should fail because it's in use.
    let removeFinanceSucceeded = false;
    try {
        await tagAdmin.trail(trailId).tags().remove("finance").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(tagAdmin);
        removeFinanceSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(removeFinanceSucceeded, false, "a tag referenced by a role or record must not be removable");

    // TagAdmin removes "legal" tag — should succeed because nothing uses it.
    await tagAdmin.trail(trailId).tags().remove("legal").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(tagAdmin);

    onChain = await admin.trail(trailId).get();
    console.log("Registry after removing \"legal\":", onChain.tags.map((t) => t.tag), "\n");
    assert.ok(onChain.tags.some((t) => t.tag === "finance"), "finance tag should still exist");
    assert.ok(!onChain.tags.some((t) => t.tag === "legal"), "legal tag should be removed");

    console.log("Tag management completed successfully.");
}
