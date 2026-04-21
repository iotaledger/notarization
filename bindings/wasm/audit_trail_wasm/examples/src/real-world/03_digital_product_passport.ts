// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Digital Product Passport Example
 *
 * Models a Digital Product Passport (DPP) for an e-bike battery, inspired by the
 * public IOTA DPP demo.
 *
 * Scope note: this example stays within the Audit Trail SDK. The demo's wider
 * IOTA stack (Identity, Hierarchies, Tokenization, and Gas Station) is mapped
 * here onto audit-trail-native concepts:
 *
 * - product identity, bill of materials, reward policy, and service history are
 *   captured as immutable audit records
 * - service-network authorization is represented through role-scoped capabilities
 * - Lifecycle Credit (LCC) payouts are documented as reward records rather than
 *   executed as token transfers
 *
 * ## Actors
 *
 * - **Manufacturer**: Creates the DPP, publishes manufacturing data, and
 *   administers roles and capabilities.
 * - **LifecycleManager**: Updates the mutable lifecycle-stage metadata.
 * - **Distributor**: Writes logistics and handover records.
 * - **Consumer**: Writes the commissioning / in-use activation record.
 * - **ServiceTechnician**: Reviews the passport, requests write access, and
 *   records the maintenance event once authorized.
 * - **Recycler**: Prepared for future end-of-life events through a
 *   recycling-scoped capability.
 * - **EPRO**: Records reward policy and the reward-payout evidence for verified
 *   maintenance.
 *
 * ## How the trail is used as a DPP
 *
 * - immutable_metadata: product identity for the battery passport
 * - updatable_metadata: current lifecycle stage
 * - record tags: manufacturing, logistics, ownership, maintenance, recycling, rewards
 * - roles and capabilities: each actor can write only its assigned slice of the lifecycle
 * - access-request flow: the technician is denied maintenance writes until the
 *   manufacturer issues the scoped capability
 * - service evidence: the maintenance event mirrors the demo's "Annual
 *   Maintenance" / "Health Snapshot" pattern with a 76% health score and a
 *   1-LCC reward record
 */

import { AuditTrailClient, CapabilityIssueOptions, Data, PermissionSet, RoleTags } from "@iota/audit-trail/node";
import { strict as assert } from "assert";
import { getFundedClient, issueTaggedRecordRole, TEST_GAS_BUDGET } from "../util";

export async function digitalProductPassport(): Promise<void> {
    console.log("=== Digital Product Passport ===\n");

    const manufacturer = await getFundedClient();
    const lifecycleManager = await getFundedClient();
    const distributor = await getFundedClient();
    const consumer = await getFundedClient();
    const serviceTechnician = await getFundedClient();
    const recycler = await getFundedClient();
    const epro = await getFundedClient();

    console.log("Manufacturer wallet:      ", manufacturer.senderAddress());
    console.log("Lifecycle manager wallet: ", lifecycleManager.senderAddress());
    console.log("Distributor wallet:       ", distributor.senderAddress());
    console.log("Consumer wallet:          ", consumer.senderAddress());
    console.log("Service technician wallet:", serviceTechnician.senderAddress());
    console.log("Recycler wallet:          ", recycler.senderAddress());
    console.log("EPRO wallet:              ", epro.senderAddress(), "\n");

    // === Create the DPP trail ===

    console.log("Creating the DPP trail for EcoBike's battery...");

    const { output: created } = await manufacturer
        .createTrail()
        .withRecordTags(["manufacturing", "logistics", "ownership", "maintenance", "recycling", "rewards"])
        .withTrailMetadata("DPP: Pro 48V Battery", "Manufacturer: EcoBike | Serial: EB-48V-2024-001337")
        .withUpdatableMetadata("Lifecycle Stage: Manufactured")
        .withInitialRecordString(
            "event=dpp_created\nproduct_name=Pro 48V Battery\nserial_number=EB-48V-2024-001337\nmanufacturer=EcoBike",
            "event:dpp_created",
            "manufacturing",
        )
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(manufacturer);

    const trailId = created.id;
    console.log("Trail created with ID", trailId, "\n");

    // === Define DPP roles and issue capabilities ===

    console.log("Configuring DPP actor roles...");

    await issueTaggedRecordRole(manufacturer, trailId, "Manufacturer", "manufacturing", manufacturer.senderAddress());
    await issueTaggedRecordRole(manufacturer, trailId, "Distributor", "logistics", distributor.senderAddress());
    await issueTaggedRecordRole(manufacturer, trailId, "Consumer", "ownership", consumer.senderAddress());
    await issueTaggedRecordRole(manufacturer, trailId, "Recycler", "recycling", recycler.senderAddress());
    await issueTaggedRecordRole(manufacturer, trailId, "EPRO", "rewards", epro.senderAddress());

    await manufacturer
        .trail(trailId)
        .access()
        .forRole("ServiceTechnician")
        .create(PermissionSet.recordAdminPermissions(), new RoleTags(["maintenance"]))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(manufacturer);

    await issueMetadataRole(manufacturer, trailId, "LifecycleManager", lifecycleManager.senderAddress());

    // === Prepare the passport with lifecycle context from the DPP demo ===

    console.log("Publishing product details, service-network context, and reward policy...");

    await manufacturer
        .trail(trailId)
        .records()
        .add(
            Data.fromString(
                "event=product_details_published\nproduct_name=Pro 48V Battery\nserial_number=EB-48V-2024-001337\nmanufacturer=EcoBike\nmanufacturer_did=did:iota:testnet:0xdc704ab63984d5763576c12ce5f62fe735766bc1fc9892a5e2a7be777a9af897\nbattery_details=48V removable e-bike battery with smart BMS\nbill_of_materials=cathode:NMC811;anode:graphite;housing:recycled_aluminum;bms:BMS-v3\ncompliance=CE,RoHS,UN38.3\nsustainability=recycled_aluminum_housing:35%\nservice_network=EcoBike certified service network",
            ),
            "event:product_details_published",
            "manufacturing",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(manufacturer);

    await epro
        .trail(trailId)
        .records()
        .add(
            Data.fromString(
                "event=reward_policy_published\nreward_type=LCC\nannual_maintenance_reward=1 LCC\nrecycling_reward=10 LCC\nfinal_owner_reward=10 LCC\nmanufacturer_return_reward=10 LCC\nend_of_life_bundle=30 LCC\nsettlement_operator=EcoCycle EPRO",
            ),
            "event:reward_policy_published",
            "rewards",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(epro);

    await lifecycleManager
        .trail(trailId)
        .updateMetadata("Lifecycle Stage: In Distribution")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lifecycleManager);

    await distributor
        .trail(trailId)
        .records()
        .add(
            Data.fromString(
                "event=distributed\nshipment_id=SHIP-EB-2026-0042\ntracking_status=Delivered to Nairobi certified service region\ntransport_certification=ADR-compliant battery transport",
            ),
            "event:distributed",
            "logistics",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(distributor);

    await lifecycleManager
        .trail(trailId)
        .updateMetadata("Lifecycle Stage: In Use")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lifecycleManager);

    await consumer
        .trail(trailId)
        .records()
        .add(
            Data.fromString(
                "event=commissioned\nowner_profile=Urban commuter fleet\nusage_status=Battery commissioned for daily e-bike service\nrepair_options=EcoBike certified annual maintenance available",
            ),
            "event:commissioned",
            "ownership",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(consumer);

    // === Technician reviews history and requests maintenance access ===

    console.log("Technician reviews the current DPP history...");

    const historyBeforeService = await serviceTechnician.trail(trailId).records().listPage(undefined, 20);
    console.log("Technician can already read", historyBeforeService.records.length, "public DPP records.\n");

    let unauthorizedWriteSucceeded = false;
    try {
        await serviceTechnician
            .trail(trailId)
            .records()
            .add(
                Data.fromString("event=unauthorized_maintenance_attempt"),
                "event:unauthorized_maintenance_attempt",
                "maintenance",
            )
            .withGasBudget(TEST_GAS_BUDGET)
            .buildAndExecute(serviceTechnician);
        unauthorizedWriteSucceeded = true;
    } catch {
        // Expected
    }
    assert.equal(
        unauthorizedWriteSucceeded,
        false,
        "maintenance writes must fail until the technician is explicitly authorized",
    );
    console.log("Maintenance write denied before access grant, as expected.\n");

    const nowMs = BigInt(Date.now());
    const technicianValidUntilMs = nowMs + BigInt(30 * 24 * 60 * 60 * 1000);

    const issuedTechnicianCap = await manufacturer
        .trail(trailId)
        .access()
        .forRole("ServiceTechnician")
        .issueCapability(
            new CapabilityIssueOptions(serviceTechnician.senderAddress(), nowMs, technicianValidUntilMs),
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(manufacturer);

    console.log(
        "Issued ServiceTechnician capability",
        issuedTechnicianCap.output.capabilityId,
        "(valid until",
        technicianValidUntilMs + ").\n",
    );

    await lifecycleManager
        .trail(trailId)
        .updateMetadata("Lifecycle Stage: Maintenance In Progress")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lifecycleManager);

    // === Perform the maintenance event described in the DPP demo ===

    console.log("Recording the annual maintenance event...");

    const maintenanceEvent = await serviceTechnician
        .trail(trailId)
        .records()
        .add(
            Data.fromString(
                "entry_type=Annual Maintenance\nservice_action=Health Snapshot\nhealth_score=76%\nfindings=Routine maintenance completed successfully\nwork_performed=Battery contacts cleaned; cell balance check passed; firmware diagnostics passed\nnext_service_due=2027-04-20",
            ),
            "event:annual_maintenance",
            "maintenance",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(serviceTechnician);

    console.log("Service technician added maintenance record", maintenanceEvent.output.sequenceNumber + ".\n");

    const rewardEvent = await epro
        .trail(trailId)
        .records()
        .add(
            Data.fromString(
                `event=lcc_reward_distributed\ntrigger_record=${maintenanceEvent.output.sequenceNumber}\nreward_type=LCC\namount=1\nreason=Annual maintenance completed\nbeneficiary=${serviceTechnician.senderAddress()}`,
            ),
            "event:lcc_reward_distributed",
            "rewards",
        )
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(epro);

    console.log(
        "EPRO added reward record",
        rewardEvent.output.sequenceNumber + " for the verified maintenance event.\n",
    );

    await lifecycleManager
        .trail(trailId)
        .updateMetadata("Lifecycle Stage: Maintained and Ready for Continued Use")
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(lifecycleManager);

    // === Verify the resulting DPP ===

    console.log("Verifying the resulting DPP...");

    const onChain = await manufacturer.trail(trailId).get();
    const firstPage = await manufacturer.trail(trailId).records().listPage(undefined, 20);

    console.log("Recorded DPP events:");
    for (const record of firstPage.records) {
        console.log(`  #${record.sequenceNumber} | tag=${record.tag} | metadata=${record.metadata}`);
    }

    assert.equal(
        firstPage.records.length,
        7,
        "expected 7 DPP records (initial + product details + reward policy + distribution + commissioning + maintenance + reward payout)",
    );
    assert.ok(
        onChain.tags.some((t) => t.tag === "maintenance")
            && onChain.tags.some((t) => t.tag === "recycling")
            && onChain.tags.some((t) => t.tag === "rewards"),
        "expected the DPP tag registry to contain maintenance, recycling, and rewards",
    );
    assert.ok(
        onChain.roles.roles.some((r) => r.name === "Manufacturer")
            && onChain.roles.roles.some((r) => r.name === "Distributor")
            && onChain.roles.roles.some((r) => r.name === "Consumer")
            && onChain.roles.roles.some((r) => r.name === "ServiceTechnician")
            && onChain.roles.roles.some((r) => r.name === "Recycler")
            && onChain.roles.roles.some((r) => r.name === "EPRO")
            && onChain.roles.roles.some((r) => r.name === "LifecycleManager"),
        "expected all DPP roles to be registered",
    );
    assert.equal(onChain.updatableMetadata, "Lifecycle Stage: Maintained and Ready for Continued Use");

    const maintenanceRecord = firstPage.records.find((record) => record.metadata === "event:annual_maintenance");
    assert.ok(maintenanceRecord, "expected the maintenance record to be present in the DPP history");

    const rewardRecord = firstPage.records.find((record) => record.metadata === "event:lcc_reward_distributed");
    assert.ok(rewardRecord, "expected the reward payout record to be present in the DPP history");

    console.log("\nDigital Product Passport scenario completed successfully.");
}

async function issueMetadataRole(
    admin: AuditTrailClient,
    trailId: string,
    roleName: string,
    issuedTo: string,
): Promise<void> {
    await admin
        .trail(trailId)
        .access()
        .forRole(roleName)
        .create(PermissionSet.metadataAdminPermissions())
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
