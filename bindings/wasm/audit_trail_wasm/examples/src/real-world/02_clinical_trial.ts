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
 * # Clinical Trial Data-Integrity Example
 *
 * Models a Phase III clinical trial where an immutable audit trail
 * guarantees data integrity, role-scoped access, and time-constrained oversight.
 *
 * - immutable_metadata: protocol identity and study description
 * - updatable_metadata: current study phase (updated as the trial progresses)
 * - record tags: enrollment, safety, efficacy, pk (added mid-study)
 * - roles and capabilities: each role writes only its designated tag
 * - time-constrained capabilities: Monitor access is windowed to the study period
 * - locking: a deletion window protects recent records; a time-lock freezes the
 *   dataset after the Data Safety Board completes its review
 * - read-only verification: a regulator inspects the trail without write access
 */
export async function clinicalTrial(): Promise<void> {
    console.log("=== Clinical Trial Data Integrity ===\n");

    const client = await getFundedClient();

    // 1. Create the trial trail
    console.log("Creating the clinical-trial audit trail...");

    const { output: created } = await client
        .createTrail()
        .withRecordTags(["enrollment", "safety", "efficacy"])
        .withTrailMetadata(
            "Protocol CTR-2026-03742",
            "Phase III: Efficacy of Drug X vs Placebo in Moderate-to-Severe Asthma",
        )
        .withUpdatableMetadata("Phase: Enrollment")
        .withLockingConfig(
            new LockingConfig(LockingWindow.withCountBased(BigInt(3)), TimeLock.withNone(), TimeLock.withNone()),
        )
        .withInitialRecordString(
            "Clinical trial CTR-2026-03742 opened for enrollment",
            "event:trial_opened",
            "enrollment",
        )
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const trailId = created.id;
    console.log("Trail created with ID", trailId, "\n");

    // 2. Define roles with tag-scoped permissions
    console.log("Defining study roles...");

    await issueTaggedRecordRole(client, trailId, "Enroller", "enrollment");
    await issueTaggedRecordRole(client, trailId, "SafetyOfficer", "safety");
    await issueTaggedRecordRole(client, trailId, "EfficacyReviewer", "efficacy");

    // Monitor can update metadata (study phase) — valid for 90 days
    await client
        .trail(trailId)
        .access()
        .forRole("Monitor")
        .create(PermissionSet.metadataAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const nowMs = BigInt(Date.now());
    const studyEndMs = nowMs + BigInt(90 * 24 * 60 * 60 * 1000);

    await client
        .trail(trailId)
        .access()
        .forRole("Monitor")
        .issueCapability(new CapabilityIssueOptions(client.senderAddress(), nowMs, studyEndMs))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    console.log("Monitor capability issued (expires at timestamp", studyEndMs + ")\n");

    // Data Safety Board can manage locking
    await client
        .trail(trailId)
        .access()
        .forRole("DataSafetyBoard")
        .create(PermissionSet.lockingAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    await client
        .trail(trailId)
        .access()
        .forRole("DataSafetyBoard")
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    // 3. Enrollment phase
    console.log("--- Enrollment Phase ---");

    const enrolled = await client
        .trail(trailId)
        .records()
        .add(Data.fromString("Patient P-101 enrolled at Site Hamburg"), "event:patient_enrolled", "enrollment")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("Enroller added record", enrolled.output.sequenceNumber + ".\n");

    // 4. Safety and efficacy records
    console.log("--- Study Data Collection ---");

    const safetyEvent = await client
        .trail(trailId)
        .records()
        .add(Data.fromString("Adverse event: mild headache reported by Patient P-101"), "event:adverse_event", "safety")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const efficacyRecord = await client
        .trail(trailId)
        .records()
        .add(
            Data.fromString("Week 12: FEV1 improvement of 320 mL over baseline for P-101"),
            "event:efficacy_observed",
            "efficacy",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    console.log(
        "SafetyOfficer added record",
        safetyEvent.output.sequenceNumber,
        ", EfficacyReviewer added record",
        efficacyRecord.output.sequenceNumber + ".\n",
    );

    // 5. Add a new tag mid-study (pharmacokinetics)
    console.log("--- Mid-Study Amendment ---");

    await client
        .trail(trailId)
        .tags()
        .add("pk")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("Added tag \"pk\" (pharmacokinetics) to the trail.");

    await issueTaggedRecordRole(client, trailId, "PkAnalyst", "pk");

    const pkRecord = await client
        .trail(trailId)
        .records()
        .add(Data.fromString("PK analysis: Cmax reached at 2.4 h, half-life 8.7 h"), "event:pk_result", "pk")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
    console.log("PkAnalyst added record", pkRecord.output.sequenceNumber + ".\n");

    // 6. Deletion window protects recent records
    console.log("--- Deletion Window Enforcement ---");

    let deleteSucceeded = false;
    try {
        await client
            .trail(trailId)
            .records()
            .delete(pkRecord.output.sequenceNumber)
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(client);
        deleteSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(
        deleteSucceeded,
        false,
        "recent records must be protected by the count-based deletion window",
    );
    console.log(
        "Record",
        pkRecord.output.sequenceNumber,
        "is within the deletion window (newest 3) and cannot be deleted.\n",
    );

    // 7. Monitor updates study phase metadata
    console.log("--- Metadata Update ---");

    await client
        .trail(trailId)
        .updateMetadata("Phase: Data Review")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const trail = await client.trail(trailId).get();
    console.log("Study phase updated to:", trail.updatableMetadata, "\n");

    // 8. Data Safety Board locks the study dataset
    console.log("--- Data Safety Board Lock ---");

    const lockUntilMs = nowMs + BigInt(365 * 24 * 60 * 60 * 1000); // 1 year from now

    await client
        .trail(trailId)
        .locking()
        .updateWriteLock(TimeLock.withUnlockAtMs(lockUntilMs))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    console.log("Write lock set to UnlockAtMs(" + lockUntilMs + ") — writes blocked until that timestamp.\n");

    // Lock trail from deletion permanently
    await client
        .trail(trailId)
        .locking()
        .updateDeleteTrailLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);

    const finalLocking = await client.trail(trailId).get();
    console.log(
        "Delete-trail lock set to",
        finalLocking.lockingConfig.deleteTrailLock.type,
        "— trail cannot be deleted.\n",
    );

    // 9. Regulator read-only verification
    console.log("--- Regulator Verification ---");

    const regulatorHandle = client.trail(trailId);
    const onChain = await regulatorHandle.get();

    console.log("Protocol:", onChain.immutableMetadata);
    console.log("Phase:  ", onChain.updatableMetadata);
    console.log("Roles:  ", onChain.roles.roles.map((r) => r.name));
    console.log("Tags:   ", onChain.tags.map((t) => t.tag));

    const firstPage = await regulatorHandle.records().listPage(undefined, 20);
    console.log("\nVerified records (" + firstPage.records.length + " total):");
    for (const record of firstPage.records) {
        console.log(`  #${record.sequenceNumber} | tag=${record.tag} | ${record.metadata}`);
    }

    // 10. Assertions
    assert.equal(firstPage.records.length, 5, "expected 5 records (initial + enrolled + safety + efficacy + pk)");
    assert.ok(onChain.tags.some((t) => t.tag === "pk"), "the 'pk' tag must exist after mid-study amendment");
    assert.equal(onChain.lockingConfig.deleteRecordWindow.type, LockingWindow.withCountBased(BigInt(3)).type);
    assert.equal(onChain.lockingConfig.deleteTrailLock.type, TimeLock.withInfinite().type);
    assert.equal(onChain.lockingConfig.writeLock.type, TimeLock.withUnlockAtMs(lockUntilMs).type);
    assert.equal(onChain.updatableMetadata, "Phase: Data Review");

    console.log("\nClinical trial data-integrity verification completed successfully.");
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
