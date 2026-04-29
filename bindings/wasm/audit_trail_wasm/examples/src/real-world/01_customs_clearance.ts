// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Customs Clearance Example
 *
 * Models a customs-clearance process for a single shipment.
 *
 * ## Actors
 *
 * - **Admin client**: Creates the trail and sets up all roles and capabilities.
 * - **DocsOperator**: Handles document submission (invoices, packing lists). Writes only
 *   `documents`-tagged records.
 * - **ExportBroker**: Files export declarations and records clearance decisions at the origin.
 *   Writes only `export`-tagged records.
 * - **ImportBroker**: Handles duty assessment and import clearance at the destination.
 *   Writes only `import`-tagged records.
 * - **Inspector**: Records the outcome of a customs physical inspection. Writes only
 *   `inspection`-tagged records; the role is created mid-process when an inspection is triggered.
 * - **Supervisor**: Updates the mutable trail metadata (processing status). No record-write
 *   permissions.
 * - **Locking admin client**: Freezes the trail once the shipment is fully cleared.
 *
 * ## How the trail is used
 *
 * - immutable_metadata: shipment and declaration identity
 * - updatable_metadata: current customs-processing status
 * - record tags: documents, export, import, inspection
 * - roles and capabilities: each operational role writes only the events it owns
 * - locking: writes are frozen once the shipment is fully cleared
 */

import {
    CapabilityIssueOptions,
    Data,
    LockingConfig,
    LockingWindow,
    PermissionSet,
    TimeLock,
} from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, issueTaggedRecordRole, TEST_GAS_BUDGET } from "../util";

export async function customsClearance(): Promise<void> {
    console.log("=== Customs Clearance ===\n");

    const adminClient = await getFundedClient();
    const docsOperator = await getFundedClient();
    const exportBroker = await getFundedClient();
    const importBroker = await getFundedClient();
    const supervisor = await getFundedClient();
    const lockingAdminClient = await getFundedClient();
    const inspector = await getFundedClient();

    // === Create the customs-clearance trail ===

    console.log("Creating a customs-clearance trail...");

    const { output: createdTrail } = await adminClient
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
        .buildAndExecute(adminClient);

    const trailId = createdTrail.id;

    // === Set up roles and capabilities for each actor ===

    await issueTaggedRecordRole(adminClient, trailId, "DocsOperator", "documents", docsOperator.senderAddress());
    await issueTaggedRecordRole(adminClient, trailId, "ExportBroker", "export", exportBroker.senderAddress());
    await issueTaggedRecordRole(adminClient, trailId, "ImportBroker", "import", importBroker.senderAddress());

    // Supervisor can update metadata.
    await adminClient
        .trail(trailId)
        .access()
        .forRole("Supervisor")
        .create(PermissionSet.metadataAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await adminClient
        .trail(trailId)
        .access()
        .forRole("Supervisor")
        .issueCapability(new CapabilityIssueOptions(supervisor.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    // LockingAdmin can manage locking.
    await adminClient
        .trail(trailId)
        .access()
        .forRole("LockingAdmin")
        .create(PermissionSet.lockingAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);
    await adminClient
        .trail(trailId)
        .access()
        .forRole("LockingAdmin")
        .issueCapability(new CapabilityIssueOptions(lockingAdminClient.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(adminClient);

    // === Document submission ===

    // Documents are stored off-chain in an access-controlled environment (e.g. a TWIN node).
    // Only the SHA-256 fingerprint is committed on-chain for tamper-evidence.
    const invoiceBytes = new TextEncoder().encode("invoice-SHP-2026-CLEAR-001-v1.pdf");
    const invoiceHash = new Uint8Array(await crypto.subtle.digest("SHA-256", invoiceBytes));
    const docsUploaded = await docsOperator
        .trail(trailId)
        .records()
        .add(Data.fromBytes(invoiceHash), "event:documents_uploaded", "documents")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(docsOperator);
    console.log("Docs operator added record", docsUploaded.output.sequenceNumber + ".\n");

    await supervisor
        .trail(trailId)
        .updateMetadata("Status: Awaiting Export Clearance")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(supervisor);

    // === Export clearance ===

    const exportFiled = await exportBroker
        .trail(trailId)
        .records()
        .add(
            Data.fromString("Export declaration filed with German customs"),
            "event:export_declaration_filed",
            "export",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(exportBroker);

    const exportCleared = await exportBroker
        .trail(trailId)
        .records()
        .add(Data.fromString("Export clearance granted by Hamburg customs office"), "event:export_cleared", "export")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(exportBroker);

    console.log(
        "Export broker added records",
        exportFiled.output.sequenceNumber,
        "and",
        exportCleared.output.sequenceNumber + ".\n",
    );

    await supervisor
        .trail(trailId)
        .updateMetadata("Status: Awaiting Import Clearance")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(supervisor);

    // === Inspection gate ===

    // The import broker does not hold an inspection-scoped capability at this point.
    // The write attempt must fail to prove that tag-based access control is enforced.
    let inspectionDenied = false;
    try {
        await importBroker
            .trail(trailId)
            .records()
            .add(
                Data.fromString("Import broker attempted to record an inspection result"),
                "event:invalid_inspection_write",
                "inspection",
            )
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(importBroker);
        inspectionDenied = true;
    } catch {
        // Expected
    }
    assert.equal(
        inspectionDenied,
        false,
        "inspection-tagged writes should fail before an inspection-scoped capability exists",
    );
    console.log("Inspection write was correctly denied before the inspector role existed.\n");

    // A customs inspection is triggered; the inspector role is created and issued mid-process.
    await issueTaggedRecordRole(adminClient, trailId, "Inspector", "inspection", inspector.senderAddress());

    const inspectionDone = await inspector
        .trail(trailId)
        .records()
        .add(
            Data.fromString("Customs inspection completed with no discrepancies"),
            "event:inspection_completed",
            "inspection",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(inspector);
    console.log("Inspector added record", inspectionDone.output.sequenceNumber + ".\n");

    // === Import clearance ===

    const dutyAssessed = await importBroker
        .trail(trailId)
        .records()
        .add(Data.fromString("Import duty assessed and paid"), "event:duty_assessed", "import")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(importBroker);

    const importCleared = await importBroker
        .trail(trailId)
        .records()
        .add(Data.fromString("Import clearance granted by Nairobi customs"), "event:import_cleared", "import")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(importBroker);

    console.log(
        "Import broker added records",
        dutyAssessed.output.sequenceNumber,
        "and",
        importCleared.output.sequenceNumber + ".\n",
    );

    await supervisor
        .trail(trailId)
        .updateMetadata("Status: Cleared")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(supervisor);

    // === Final lock and verification ===

    await lockingAdminClient
        .trail(trailId)
        .locking()
        .updateWriteLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lockingAdminClient);

    const trailAfterLock = await adminClient.trail(trailId).get();
    console.log("Write lock after clearance:", trailAfterLock.lockingConfig.writeLock, "\n");

    let lateWriteSucceeded = false;
    try {
        await docsOperator
            .trail(trailId)
            .records()
            .add(Data.fromString("Late customs note after the case was closed"), "event:late_note", "documents")
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(docsOperator);
        lateWriteSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(lateWriteSucceeded, false, "cleared customs trail should reject late writes after the final lock");

    const firstRecordsPage = await adminClient.trail(trailId).records().listPage(undefined, 20);
    console.log("Recorded customs events:");
    for (const record of firstRecordsPage.records) {
        console.log(`  #${record.sequenceNumber} | ${record.data} | tag=${record.tag} | ${record.metadata}`);
    }

    assert.equal(
        firstRecordsPage.records.length,
        7,
        "expected 7 customs records including the initial case-opened record",
    );

    const finalTrail = await adminClient.trail(trailId).get();
    assert.equal(finalTrail.updatableMetadata, "Status: Cleared", "customs case should finish in cleared state");

    console.log("\nCustoms clearance completed successfully.");
}
