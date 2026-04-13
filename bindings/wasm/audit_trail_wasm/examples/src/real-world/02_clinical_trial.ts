// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Clinical Trial Data-Integrity Example
 *
 * Models a Phase III clinical trial where an immutable audit trail
 * guarantees data integrity, role-scoped access, and time-constrained oversight.
 *
 * ## Actors
 *
 * - **Admin**: Creates the trail and sets up all roles and capabilities.
 * - **Enroller**: Writes enrollment events. Restricted to the `enrollment` tag.
 * - **SafetyOfficer**: Records adverse events and safety observations. Restricted to `safety`.
 * - **EfficacyReviewer**: Records treatment outcomes. Restricted to `efficacy`.
 * - **PkAnalyst**: Records pharmacokinetic results. Restricted to the `pk` tag that is added
 *   mid-study when a PK sub-study is initiated.
 * - **Monitor**: Updates the mutable study-phase metadata. Access is time-windowed to the
 *   active study period (90 days from now).
 * - **DataSafetyBoard**: Controls write and delete locks. Freezes the dataset after review.
 * - **Regulator**: Read-only verifier. In production this would use `AuditTrailClientReadOnly`
 *   (no signing key); here a funded client is used to keep the example self-contained.
 *
 * ## How the trail is used
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

import {
    AuditTrailClient,
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

export async function clinicalTrial(): Promise<void> {
    console.log("=== Clinical Trial Data Integrity ===\n");

    const admin = await getFundedClient();
    const enroller = await getFundedClient();
    const safetyOfficer = await getFundedClient();
    const efficacyReviewer = await getFundedClient();
    const pkAnalyst = await getFundedClient();
    const monitor = await getFundedClient();
    const dataSafetyBoard = await getFundedClient();
    const regulator = await getFundedClient();

    // === Create the clinical-trial trail ===

    console.log("Creating the clinical-trial audit trail...");

    const { output: created } = await admin
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
        .buildAndExecute(admin);

    const trailId = created.id;
    console.log("Trail created with ID", trailId, "\n");

    // === Define roles with tag-scoped permissions ===

    console.log("Defining study roles...");

    await issueTaggedRecordRole(admin, trailId, "Enroller", "enrollment", enroller.senderAddress());
    await issueTaggedRecordRole(admin, trailId, "SafetyOfficer", "safety", safetyOfficer.senderAddress());
    await issueTaggedRecordRole(admin, trailId, "EfficacyReviewer", "efficacy", efficacyReviewer.senderAddress());

    // Monitor can update metadata (study phase) — valid for 90 days.
    await admin
        .trail(trailId)
        .access()
        .forRole("Monitor")
        .create(PermissionSet.metadataAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    const nowMs = BigInt(Date.now());
    const studyEndMs = nowMs + BigInt(90 * 24 * 60 * 60 * 1000);

    await admin
        .trail(trailId)
        .access()
        .forRole("Monitor")
        .issueCapability(new CapabilityIssueOptions(monitor.senderAddress(), nowMs, studyEndMs))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    console.log("Monitor capability issued (expires at timestamp", studyEndMs + ")\n");

    // Data Safety Board can manage locking.
    await admin
        .trail(trailId)
        .access()
        .forRole("DataSafetyBoard")
        .create(PermissionSet.lockingAdminPermissions())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await admin
        .trail(trailId)
        .access()
        .forRole("DataSafetyBoard")
        .issueCapability(new CapabilityIssueOptions(dataSafetyBoard.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);

    // === Enrollment phase ===

    console.log("--- Enrollment Phase ---");

    const enrolled = await enroller
        .trail(trailId)
        .records()
        .add(Data.fromString("Patient P-101 enrolled at Site Hamburg"), "event:patient_enrolled", "enrollment")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(enroller);
    console.log("Enroller added record", enrolled.output.sequenceNumber + ".\n");

    // === Study data collection ===

    console.log("--- Study Data Collection ---");

    const safetyEvent = await safetyOfficer
        .trail(trailId)
        .records()
        .add(
            Data.fromString("Adverse event: mild headache reported by Patient P-101"),
            "event:adverse_event",
            "safety",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(safetyOfficer);

    const efficacyRecord = await efficacyReviewer
        .trail(trailId)
        .records()
        .add(
            Data.fromString("Week 12: FEV1 improvement of 320 mL over baseline for P-101"),
            "event:efficacy_observed",
            "efficacy",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(efficacyReviewer);

    console.log(
        "SafetyOfficer added record",
        safetyEvent.output.sequenceNumber,
        ", EfficacyReviewer added record",
        efficacyRecord.output.sequenceNumber + ".\n",
    );

    // === Mid-study amendment: add pharmacokinetics tag ===

    console.log("--- Mid-Study Amendment ---");

    await admin.trail(trailId).tags().add("pk").withGasBudget(TEST_GAS_BUDGET).buildAndExecute(admin);
    console.log("Added tag \"pk\" (pharmacokinetics) to the trail.");

    await issueTaggedRecordRole(admin, trailId, "PkAnalyst", "pk", pkAnalyst.senderAddress());

    const pkRecord = await pkAnalyst
        .trail(trailId)
        .records()
        .add(Data.fromString("PK analysis: Cmax reached at 2.4 h, half-life 8.7 h"), "event:pk_result", "pk")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(pkAnalyst);
    console.log("PkAnalyst added record", pkRecord.output.sequenceNumber + ".\n");

    // === Deletion window enforcement ===

    console.log("--- Deletion Window Enforcement ---");

    // The PkAnalyst has RecordAdmin permissions, but the count-based deletion window
    // protects the newest 3 records, so this attempt must fail.
    let deleteSucceeded = false;
    try {
        await pkAnalyst
            .trail(trailId)
            .records()
            .delete(pkRecord.output.sequenceNumber)
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(pkAnalyst);
        deleteSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(deleteSucceeded, false, "recent records must be protected by the count-based deletion window");
    console.log(
        "Record",
        pkRecord.output.sequenceNumber,
        "is within the deletion window (newest 3) and cannot be deleted.\n",
    );

    // === Metadata update (Monitor) ===

    console.log("--- Metadata Update ---");

    await monitor
        .trail(trailId)
        .updateMetadata("Phase: Data Review")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(monitor);

    const trail = await admin.trail(trailId).get();
    console.log("Study phase updated to:", trail.updatableMetadata, "\n");

    // === Data Safety Board locks the study dataset ===

    console.log("--- Data Safety Board Lock ---");

    const lockUntilMs = nowMs + BigInt(365 * 24 * 60 * 60 * 1000); // 1 year from now

    await dataSafetyBoard
        .trail(trailId)
        .locking()
        .updateWriteLock(TimeLock.withUnlockAtMs(lockUntilMs))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(dataSafetyBoard);

    console.log("Write lock set to UnlockAtMs(" + lockUntilMs + ") — writes blocked until that timestamp.\n");

    // Lock trail from deletion permanently.
    await dataSafetyBoard
        .trail(trailId)
        .locking()
        .updateDeleteTrailLock(TimeLock.withInfinite())
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(dataSafetyBoard);

    const finalLocking = await admin.trail(trailId).get();
    console.log(
        "Delete-trail lock set to",
        finalLocking.lockingConfig.deleteTrailLock.type,
        "— trail cannot be deleted.\n",
    );

    // === Regulator read-only verification ===

    console.log("--- Regulator Verification ---");

    // In production the regulator would use AuditTrailClientReadOnly (no signing key).
    // Here a funded client is used to keep the example self-contained.
    const regulatorHandle = regulator.trail(trailId);
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

    assert.equal(firstPage.records.length, 5, "expected 5 records (initial + enrolled + safety + efficacy + pk)");
    assert.ok(onChain.tags.some((t) => t.tag === "pk"), "the 'pk' tag must exist after mid-study amendment");
    assert.equal(onChain.lockingConfig.deleteRecordWindow.type, LockingWindow.withCountBased(BigInt(3)).type);
    assert.equal(onChain.lockingConfig.deleteTrailLock.type, TimeLock.withInfinite().type);
    assert.equal(onChain.lockingConfig.writeLock.type, TimeLock.withUnlockAtMs(lockUntilMs).type);
    assert.equal(onChain.updatableMetadata, "Phase: Data Review");

    console.log("\nClinical trial data-integrity verification completed successfully.");
}

async function issueTaggedRecordRole(
    admin: AuditTrailClient,
    trailId: string,
    roleName: string,
    tag: string,
    issuedTo: string,
): Promise<void> {
    await admin
        .trail(trailId)
        .access()
        .forRole(roleName)
        .create(PermissionSet.recordAdminPermissions(), new RoleTags([tag]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
    await admin
        .trail(trailId)
        .access()
        .forRole(roleName)
        .issueCapability(new CapabilityIssueOptions(issuedTo))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(admin);
}
