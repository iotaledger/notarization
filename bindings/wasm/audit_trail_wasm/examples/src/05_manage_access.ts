// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * ## Actors
 *
 * - **Admin client**: Creates and updates roles, issues capabilities, revokes and destroys them,
 *   and finally deletes the role once it is no longer needed.
 * - **Operations user client**: The subject of all capability issuance. Capabilities are bound to
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

    // `adminClient` manages roles and the full capability lifecycle.
    // `operationsUserClient` is the target of constrained capability issuance.
    const adminClient = await getFundedClient();
    const operationsUserClient = await getFundedClient();

    const { output: createdTrail } = await createTrailWithSeedRecord(adminClient);
    const trailId = createdTrail.id;

    const createdOperationsRole = await adminClient
        .trail(trailId)
        .access()
        .forRole("Operations")
        .create(PermissionSet.recordAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    console.log("Created role:", createdOperationsRole.output.role, "\n");

    // 2. Update the role permissions
    const updatedPermissionValues = [
        Permission.AddRecord,
        Permission.DeleteRecord,
        Permission.DeleteAllRecords,
    ];
    const updatedPermissions = new PermissionSet(updatedPermissionValues);
    const updatedOperationsRole = await adminClient
        .trail(trailId)
        .access()
        .forRole("Operations")
        .updatePermissions(updatedPermissions)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    console.log(
        "Updated role permissions:",
        updatedOperationsRole.output.permissions.permissions.map((p) => p.toString()),
    );

    // 3. Issue a capability bound to operationsUserClient's address and expiry window.
    const constrainedOperationsCapability = await adminClient
        .trail(trailId)
        .access()
        .forRole("Operations")
        .issueCapability(
            new CapabilityIssueOptions(operationsUserClient.senderAddress(), undefined, BigInt(4_102_444_800_000)),
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    console.log("\nIssued constrained capability:");
    console.log("  id =", constrainedOperationsCapability.output.capabilityId);
    console.log("  issued_to =", constrainedOperationsCapability.output.issuedTo);
    console.log("  valid_until =", constrainedOperationsCapability.output.validUntil, "\n");

    // Verify the on-chain role matches the updated permissions.
    const onChainTrail = await adminClient.trail(trailId).get();
    const operationsRole = onChainTrail.roles.roles.find((r) => r.name === "Operations");
    assert.ok(operationsRole, "Operations role must exist");
    const operationsPermissionSet = new Set(operationsRole?.permissions.map((p) => p.toString()));
    for (const perm of updatedPermissionValues) {
        assert(operationsPermissionSet.has(perm.toString()), `role should contain ${perm}`);
    }

    // 4. Revoke the constrained capability.
    await adminClient
        .trail(trailId)
        .access()
        .revokeCapability(
            constrainedOperationsCapability.output.capabilityId,
            constrainedOperationsCapability.output.validUntil,
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    console.log("Revoked capability", constrainedOperationsCapability.output.capabilityId, "\n");

    // 5. Issue a disposable capability to the Admin actor and destroy it.
    // destroyCapability consumes the capability object, so the signer must own it.
    // The capability is issued to adminClient so adminClient can destroy it directly.
    const disposableOperationsCapability = await adminClient
        .trail(trailId)
        .access()
        .forRole("Operations")
        .issueCapability(new CapabilityIssueOptions(adminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await adminClient
        .trail(trailId)
        .access()
        .destroyCapability(disposableOperationsCapability.output.capabilityId)
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    console.log("Destroyed capability", disposableOperationsCapability.output.capabilityId, "\n");

    // 6. Clean up the revoked-capability registry entry so the role can be removed.
    await adminClient
        .trail(trailId)
        .access()
        .cleanupRevokedCapabilities()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    console.log("Cleaned up revoked capability registry entries.\n");

    // 7. Delete the role.
    await adminClient
        .trail(trailId)
        .access()
        .forRole("Operations")
        .delete()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    const trailAfterDelete = await adminClient.trail(trailId).get();
    const operationsRoleAfterDelete = trailAfterDelete.roles.roles.find((r) => r.name === "Operations");
    assert.equal(operationsRoleAfterDelete, undefined, "role should be removed from the trail");

    console.log("Removed the custom role after its capability lifecycle completed.");
}
