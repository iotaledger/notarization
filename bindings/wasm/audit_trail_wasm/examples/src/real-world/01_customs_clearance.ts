// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import {
    CapabilityIssueOptions,
    Data,
    LockingConfig,
    LockingWindow,
    PermissionSet,
    RoleTags,
    TimeLock,
} from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, TEST_GAS_BUDGET } from "../util";

/**
 * # Customs Clearance Example
 *
 * Models a customs-clearance process for a single shipment.
 *
 * - immutable_metadata: shipment and declaration identity
 * - updatable_metadata: current customs-processing status
 * - record tags: documents, export, import, inspection
 * - roles and capabilities: each operational role writes only the events it owns
 * - locking: writes are frozen once the shipment is fully cleared
 */
export async function customsClearance(): Promise<void> {
    console.log("=== Customs Clearance ===\n");

    const client = await getFundedClient();

    // 1. Create the trail
    console.log("Creating a customs-clearance trail...");

    const { output: created } = await client
        .createTrail()
        .withRecordTags(["documents", "export", "import", "inspection"])
        .withTrailMetadata(
            "Shipment SHP-2026-CLEAR-001",
            "Route: Hamburg, Germany -> Nairobi, Kenya | Declaration: DEC-2026-44017",
        )
        .withUpdatableMetadata("Status: Documents Pending")
        .withLockingConfig(
            new LockingConfig(LockingWindow.withCountBased(BigInt(2)), TimeLock.withNone(), TimeLock.withNone()),
        )
        .withInitialRecordString("Customs clearance case opened for inbound shipment", "event:case_opened", "documents")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const trailId = created.id;

    // 2. Create tag-scoped roles
    await issueTaggedRecordRole(client, trailId, "DocsOperator", "documents");
    await issueTaggedRecordRole(client, trailId, "ExportBroker", "export");
    await issueTaggedRecordRole(client, trailId, "ImportBroker", "import");

    // Supervisor can update metadata
    await client
        .trail(trailId)
        .access()
        .forRole("Supervisor")
        .create(PermissionSet.metadataAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await client
        .trail(trailId)
        .access()
        .forRole("Supervisor")
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // LockingAdmin can manage locking
    await client
        .trail(trailId)
        .access()
        .forRole("LockingAdmin")
        .create(PermissionSet.lockingAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await client
        .trail(trailId)
        .access()
        .forRole("LockingAdmin")
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // 3. Upload documents
    const docsUploaded = await client
        .trail(trailId)
        .records()
        .add(Data.fromString("Commercial invoice and packing list uploaded"), "event:documents_uploaded", "documents")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("Docs operator added record", docsUploaded.output.sequenceNumber + ".\n");

    // 4. Update metadata — awaiting export clearance
    await client
        .trail(trailId)
        .updateMetadata("Status: Awaiting Export Clearance")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // 5. Export clearance
    const exportFiled = await client
        .trail(trailId)
        .records()
        .add(Data.fromString("Export declaration filed with German customs"), "event:export_declaration_filed", "export")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const exportCleared = await client
        .trail(trailId)
        .records()
        .add(Data.fromString("Export clearance granted by Hamburg customs office"), "event:export_cleared", "export")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    console.log(
        "Export broker added records",
        exportFiled.output.sequenceNumber,
        "and",
        exportCleared.output.sequenceNumber + ".\n",
    );

    // 6. Update metadata — awaiting import clearance
    await client
        .trail(trailId)
        .updateMetadata("Status: Awaiting Import Clearance")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // 7. Attempt an inspection write before the inspector role exists
    let inspectionDenied = false;
    try {
        await client
            .trail(trailId)
            .records()
            .add(Data.fromString("Import broker attempted to record an inspection result"), "event:invalid_inspection_write", "inspection")
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(client);
        inspectionDenied = true;
    } catch {
        // Expected
    }
    assert.equal(inspectionDenied, false, "inspection-tagged writes should fail before an inspection-scoped capability exists");
    console.log("Inspection write was correctly denied before the inspector role existed.\n");

    // 8. Create inspector role and add inspection record
    await issueTaggedRecordRole(client, trailId, "Inspector", "inspection");

    const inspectionDone = await client
        .trail(trailId)
        .records()
        .add(Data.fromString("Customs inspection completed with no discrepancies"), "event:inspection_completed", "inspection")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("Inspector added record", inspectionDone.output.sequenceNumber + ".\n");

    // 9. Import clearance
    const dutyAssessed = await client
        .trail(trailId)
        .records()
        .add(Data.fromString("Import duty assessed and paid"), "event:duty_assessed", "import")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const importCleared = await client
        .trail(trailId)
        .records()
        .add(Data.fromString("Import clearance granted by Nairobi customs"), "event:import_cleared", "import")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    console.log(
        "Import broker added records",
        dutyAssessed.output.sequenceNumber,
        "and",
        importCleared.output.sequenceNumber + ".\n",
    );

    // 10. Mark as cleared
    await client
        .trail(trailId)
        .updateMetadata("Status: Cleared")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // 11. Freeze writes
    await client
        .trail(trailId)
        .locking()
        .updateWriteLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const afterLock = await client.trail(trailId).get();
    console.log("Write lock after clearance:", afterLock.lockingConfig.writeLock, "\n");

    // 12. Verify that late writes are rejected
    let lateWriteSucceeded = false;
    try {
        await client
            .trail(trailId)
            .records()
            .add(Data.fromString("Late customs note after the case was closed"), "event:late_note", "documents")
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(client);
        lateWriteSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(lateWriteSucceeded, false, "cleared customs trail should reject late writes after the final lock");

    // 13. List all records
    const firstPage = await client.trail(trailId).records().listPage(undefined, 20);
    console.log("Recorded customs events:");
    for (const record of firstPage.records) {
        console.log(`  #${record.sequenceNumber} | ${record.data} | tag=${record.tag} | ${record.metadata}`);
    }

    assert.equal(firstPage.records.length, 7, "expected 7 customs records including the initial case-opened record");

    const trailState = await client.trail(trailId).get();
    assert.equal(trailState.updatableMetadata, "Status: Cleared", "customs case should finish in cleared state");

    console.log("\nCustoms clearance completed successfully.");
}

async function issueTaggedRecordRole(
    client: any,
    trailId: string,
    roleName: string,
    tag: string,
): Promise<void> {
    await client
        .trail(trailId)
        .access()
        .forRole(roleName)
        .create(PermissionSet.recordAdminPermissions(), new RoleTags([tag]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await client
        .trail(trailId)
        .access()
        .forRole(roleName)
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
}
