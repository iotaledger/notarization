// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin**: Creates and updates roles, issues capabilities, revokes and destroys them,
 *   and finally deletes the role once it is no longer needed.
 * - **OperationsUser**: The subject of all capability issuance. Capabilities are bound to
 *   this address to demonstrate that revocation immediately blocks their access.
 *
 * Demonstrates how to:
 * 1. Create and update a custom role.
 * 2. Issue a constrained capability for that role.
 * 3. Revoke one capability and destroy another.
 * 4. Remove the role after its capabilities are no longer needed.
 */

import { CapabilityIssueOptions, Permission, PermissionSet } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { createTrailWithSeedRecord, getFundedClient, TEST_GAS_BUDGET } from "./util";

export async function manageAccess(): Promise<void> {
    console.log("=== Audit Trail: Manage Access ===\n");

    // `admin` manages roles and the full capability lifecycle.
    // `operationsUser` is the target of all capability issuance.
    const admin = await getFundedClient();
    const operationsUser = await getFundedClient();

    const { output: trail } = await createTrailWithSeedRecord(admin);
    const trailId = trail.id;
    const trailHandle = admin.trail(trailId);
    const role = trailHandle.access().forRole("Operations");

    // 1. Create the role
    const createdRole = await role
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    console.log("Created role:", createdRole.output.role, "\n");

    // 2. Update the role permissions
    const updatedPermissionValues = [
        Permission.AddRecord,
        Permission.DeleteRecord,
        Permission.DeleteAllRecords,
    ];
    const updatedPermissions = new PermissionSet(updatedPermissionValues);
    const updatedRole = await role
        .updatePermissions(updatedPermissions)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    console.log("Updated role permissions:", updatedRole.output.permissions.permissions.map((p) => p.toString()));

    // 3. Issue a constrained capability bound to operationsUser's address.
    const constrainedCap = await role
        .issueCapability(new CapabilityIssueOptions(operationsUser.senderAddress(), undefined, BigInt(4_102_444_800_000)))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    console.log("\nIssued constrained capability:");
    console.log("  id =", constrainedCap.output.capabilityId);
    console.log("  issued_to =", constrainedCap.output.issuedTo);
    console.log("  valid_until =", constrainedCap.output.validUntil, "\n");

    // Verify the on-chain role matches the updated permissions.
    const onChain = await trailHandle.get();
    const opsRole = onChain.roles.roles.find((r) => r.name === "Operations");
    assert.ok(opsRole, "Operations role must exist");
    const opsPermSet = new Set(opsRole.permissions.map((p) => p.toString()));
    for (const perm of updatedPermissionValues) {
        assert(opsPermSet.has(perm.toString()), `role should contain ${perm}`);
    }

    // 4. Revoke the constrained capability.
    await trailHandle
        .access()
        .revokeCapability(constrainedCap.output.capabilityId, constrainedCap.output.validUntil)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    console.log("Revoked capability", constrainedCap.output.capabilityId, "\n");

    // 5. Issue a disposable capability and destroy it.
    const disposableCap = await role
        .issueCapability(new CapabilityIssueOptions(operationsUser.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await trailHandle
        .access()
        .destroyCapability(disposableCap.output.capabilityId)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    console.log("Destroyed capability", disposableCap.output.capabilityId, "\n");

    // 6. Clean up the revoked-capability registry entry so the role can be removed.
    await trailHandle
        .access()
        .cleanupRevokedCapabilities()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    console.log("Cleaned up revoked capability registry entries.\n");

    // 7. Delete the role.
    await role.delete().withGasBudget(TEST_GAS_BUDGET).buildAndExecute(admin);
    const afterDelete = await trailHandle.get();
    const opsRoleAfterDelete = afterDelete.roles.roles.find((r) => r.name === "Operations");
    assert.equal(opsRoleAfterDelete, undefined, "role should be removed from the trail");

    console.log("Removed the custom role after its capability lifecycle completed.");
}
