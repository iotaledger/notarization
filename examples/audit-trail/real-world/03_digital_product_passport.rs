// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Digital Product Passport Example
//!
//! This example models a Digital Product Passport (DPP) for an e-bike battery,
//! inspired by the public IOTA DPP demo.
//!
//! Scope note: this example stays within the Audit Trail SDK. The demo's wider
//! IOTA stack (Identity, Hierarchies, Tokenization, and Gas Station) is mapped
//! here onto audit-trail-native concepts:
//!
//! - product identity, bill of materials, reward policy, and service history are captured as immutable audit records
//! - service-network authorization is represented through role-scoped capabilities
//! - Lifecycle Credit (LCC) payouts are documented as reward records rather than executed as token transfers
//!
//! ## Actors
//!
//! - **Manufacturer**: Creates the DPP, publishes manufacturing data, and administers roles and capabilities.
//! - **LifecycleManager**: Updates the mutable lifecycle-stage metadata.
//! - **Distributor**: Writes logistics and handover records.
//! - **Consumer**: Writes the commissioning / in-use activation record.
//! - **ServiceTechnician**: Reviews the passport, requests write access, and records the maintenance event once
//!   authorized.
//! - **Recycler**: Prepared for future end-of-life events through a recycling-scoped capability.
//! - **EPRO**: Records reward policy and the reward-payout evidence for verified maintenance.
//!
//! ## How the trail is used as a DPP
//!
//! - `immutable_metadata`: product identity for the battery passport
//! - `updatable_metadata`: current lifecycle stage
//! - record tags: `manufacturing`, `logistics`, `ownership`, `maintenance`, `recycling`, `rewards`
//! - roles and capabilities: each actor can write only its assigned slice of the lifecycle
//! - access-request flow: the technician is denied maintenance writes until the manufacturer issues the scoped
//!   capability
//! - service evidence: the maintenance event mirrors the demo's "Annual Maintenance" / "Health Snapshot" pattern with a
//!   76% health score and a 1-LCC reward record

use anyhow::{Result, ensure};
use audit_trail::AuditTrailClient;
use audit_trail::core::types::{
    CapabilityIssueOptions, Data, ImmutableMetadata, InitialRecord, PermissionSet, RoleTags,
};
use examples::{get_funded_audit_trail_client, issue_tagged_record_role};
use iota_sdk::types::base_types::{IotaAddress, ObjectID};
use product_common::core_client::CoreClient;
use product_common::test_utils::InMemSigner;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Digital Product Passport ===\n");

    let manufacturer = get_funded_audit_trail_client().await?;
    let lifecycle_manager = get_funded_audit_trail_client().await?;
    let distributor = get_funded_audit_trail_client().await?;
    let consumer = get_funded_audit_trail_client().await?;
    let service_technician = get_funded_audit_trail_client().await?;
    let recycler = get_funded_audit_trail_client().await?;
    let epro = get_funded_audit_trail_client().await?;

    println!("Manufacturer wallet:       {}", manufacturer.sender_address());
    println!("Lifecycle manager wallet:  {}", lifecycle_manager.sender_address());
    println!("Distributor wallet:        {}", distributor.sender_address());
    println!("Consumer wallet:           {}", consumer.sender_address());
    println!("Service technician wallet: {}", service_technician.sender_address());
    println!("Recycler wallet:           {}", recycler.sender_address());
    println!("EPRO wallet:               {}\n", epro.sender_address());

    // ---------------------------------------------------------------------
    // 1. Create the DPP audit trail
    // ---------------------------------------------------------------------
    println!("Creating the DPP trail for EcoBike's battery...");

    let created = manufacturer
        .create_trail()
        .with_record_tags([
            "manufacturing",
            "logistics",
            "ownership",
            "maintenance",
            "recycling",
            "rewards",
        ])
        .with_trail_metadata(ImmutableMetadata::new(
            "DPP: Pro 48V Battery".to_string(),
            Some("Manufacturer: EcoBike | Serial: EB-48V-2024-001337".to_string()),
        ))
        .with_updatable_metadata("Lifecycle Stage: Manufactured")
        .with_initial_record(InitialRecord::new(
            Data::text(
                "event=dpp_created\nproduct_name=Pro 48V Battery\nserial_number=EB-48V-2024-001337\nmanufacturer=EcoBike",
            ),
            Some("event:dpp_created".to_string()),
            Some("manufacturing".to_string()),
        ))
        .finish()
        .build_and_execute(&manufacturer)
        .await?
        .output;

    let trail_id = created.trail_id;
    println!("Trail created with ID {trail_id}\n");

    // ---------------------------------------------------------------------
    // 2. Define DPP roles and issue capabilities
    // ---------------------------------------------------------------------
    println!("Configuring DPP actor roles...");

    issue_tagged_record_role(
        &manufacturer,
        trail_id,
        "Manufacturer",
        "manufacturing",
        manufacturer.sender_address(),
    )
    .await?;
    issue_tagged_record_role(
        &manufacturer,
        trail_id,
        "Distributor",
        "logistics",
        distributor.sender_address(),
    )
    .await?;
    issue_tagged_record_role(
        &manufacturer,
        trail_id,
        "Consumer",
        "ownership",
        consumer.sender_address(),
    )
    .await?;
    issue_tagged_record_role(
        &manufacturer,
        trail_id,
        "Recycler",
        "recycling",
        recycler.sender_address(),
    )
    .await?;
    issue_tagged_record_role(&manufacturer, trail_id, "EPRO", "rewards", epro.sender_address()).await?;

    manufacturer
        .trail(trail_id)
        .access()
        .for_role("ServiceTechnician")
        .create(
            PermissionSet::record_admin_permissions(),
            Some(RoleTags::new(["maintenance"])),
        )
        .build_and_execute(&manufacturer)
        .await?;

    issue_metadata_role(
        &manufacturer,
        trail_id,
        "LifecycleManager",
        lifecycle_manager.sender_address(),
    )
    .await?;

    // ---------------------------------------------------------------------
    // 3. Prepare the passport with lifecycle context from the DPP demo
    // ---------------------------------------------------------------------
    println!("Publishing product details, service-network context, and reward policy...");

    manufacturer
        .trail(trail_id)
        .records()
        .add(
            Data::text(
                "event=product_details_published\nproduct_name=Pro 48V Battery\nserial_number=EB-48V-2024-001337\nmanufacturer=EcoBike\nmanufacturer_did=did:iota:testnet:0xdc704ab63984d5763576c12ce5f62fe735766bc1fc9892a5e2a7be777a9af897\nbattery_details=48V removable e-bike battery with smart BMS\nbill_of_materials=cathode:NMC811;anode:graphite;housing:recycled_aluminum;bms:BMS-v3\ncompliance=CE,RoHS,UN38.3\nsustainability=recycled_aluminum_housing:35%\nservice_network=EcoBike certified service network",
            ),
            Some("event:product_details_published".to_string()),
            Some("manufacturing".to_string()),
        )
        .build_and_execute(&manufacturer)
        .await?;

    epro.trail(trail_id)
        .records()
        .add(
            Data::text(
                "event=reward_policy_published\nreward_type=LCC\nannual_maintenance_reward=1 LCC\nrecycling_reward=10 LCC\nfinal_owner_reward=10 LCC\nmanufacturer_return_reward=10 LCC\nend_of_life_bundle=30 LCC\nsettlement_operator=EcoCycle EPRO",
            ),
            Some("event:reward_policy_published".to_string()),
            Some("rewards".to_string()),
        )
        .build_and_execute(&epro)
        .await?;

    lifecycle_manager
        .trail(trail_id)
        .update_metadata(Some("Lifecycle Stage: In Distribution".to_string()))
        .build_and_execute(&lifecycle_manager)
        .await?;

    distributor
        .trail(trail_id)
        .records()
        .add(
            Data::text(
                "event=distributed\nshipment_id=SHIP-EB-2026-0042\ntracking_status=Delivered to Nairobi certified service region\ntransport_certification=ADR-compliant battery transport",
            ),
            Some("event:distributed".to_string()),
            Some("logistics".to_string()),
        )
        .build_and_execute(&distributor)
        .await?;

    lifecycle_manager
        .trail(trail_id)
        .update_metadata(Some("Lifecycle Stage: In Use".to_string()))
        .build_and_execute(&lifecycle_manager)
        .await?;

    consumer
        .trail(trail_id)
        .records()
        .add(
            Data::text(
                "event=commissioned\nowner_profile=Urban commuter fleet\nusage_status=Battery commissioned for daily e-bike service\nrepair_options=EcoBike certified annual maintenance available",
            ),
            Some("event:commissioned".to_string()),
            Some("ownership".to_string()),
        )
        .build_and_execute(&consumer)
        .await?;

    // ---------------------------------------------------------------------
    // 4. Technician reviews history and requests maintenance access
    // ---------------------------------------------------------------------
    println!("Technician reviews the current DPP history...");

    let history_before_service = service_technician.trail(trail_id).records().list_page(None, 20).await?;
    println!(
        "Technician can already read {} public DPP records.\n",
        history_before_service.records.len()
    );

    let denied_before_grant = service_technician
        .trail(trail_id)
        .records()
        .add(
            Data::text("event=unauthorized_maintenance_attempt"),
            Some("event:unauthorized_maintenance_attempt".to_string()),
            Some("maintenance".to_string()),
        )
        .build_and_execute(&service_technician)
        .await;

    ensure!(
        denied_before_grant.is_err(),
        "maintenance writes must fail until the technician is explicitly authorized"
    );
    println!("Maintenance write denied before access grant, as expected.\n");

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64;
    let technician_valid_until_ms = now_ms + 30 * 24 * 60 * 60 * 1000;

    let issued_technician_cap = manufacturer
        .trail(trail_id)
        .access()
        .for_role("ServiceTechnician")
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(service_technician.sender_address()),
            valid_from_ms: Some(now_ms),
            valid_until_ms: Some(technician_valid_until_ms),
        })
        .build_and_execute(&manufacturer)
        .await?
        .output;

    println!(
        "Issued ServiceTechnician capability {} (valid until {}).\n",
        issued_technician_cap.capability_id, technician_valid_until_ms
    );

    lifecycle_manager
        .trail(trail_id)
        .update_metadata(Some("Lifecycle Stage: Maintenance In Progress".to_string()))
        .build_and_execute(&lifecycle_manager)
        .await?;

    // ---------------------------------------------------------------------
    // 5. Perform the maintenance event described in the DPP demo
    // ---------------------------------------------------------------------
    println!("Recording the annual maintenance event...");

    let maintenance_event = service_technician
        .trail(trail_id)
        .records()
        .add(
            Data::text(
                "entry_type=Annual Maintenance\nservice_action=Health Snapshot\nhealth_score=76%\nfindings=Routine maintenance completed successfully\nwork_performed=Battery contacts cleaned; cell balance check passed; firmware diagnostics passed\nnext_service_due=2027-04-20",
            ),
            Some("event:annual_maintenance".to_string()),
            Some("maintenance".to_string()),
        )
        .build_and_execute(&service_technician)
        .await?
        .output;

    println!(
        "Service technician added maintenance record #{}.\n",
        maintenance_event.sequence_number
    );

    let reward_event = epro
        .trail(trail_id)
        .records()
        .add(
            Data::text(format!(
                "event=lcc_reward_distributed\ntrigger_record={}\nreward_type=LCC\namount=1\nreason=Annual maintenance completed\nbeneficiary={}",
                maintenance_event.sequence_number,
                service_technician.sender_address()
            )),
            Some("event:lcc_reward_distributed".to_string()),
            Some("rewards".to_string()),
        )
        .build_and_execute(&epro)
        .await?
        .output;

    println!(
        "EPRO added reward record #{} for the verified maintenance event.\n",
        reward_event.sequence_number
    );

    lifecycle_manager
        .trail(trail_id)
        .update_metadata(Some(
            "Lifecycle Stage: Maintained and Ready for Continued Use".to_string(),
        ))
        .build_and_execute(&lifecycle_manager)
        .await?;

    // ---------------------------------------------------------------------
    // 6. Verify the prepared DPP state
    // ---------------------------------------------------------------------
    println!("Verifying the resulting DPP...");

    let on_chain = manufacturer.trail(trail_id).get().await?;
    let first_page = manufacturer.trail(trail_id).records().list_page(None, 20).await?;

    println!("Recorded DPP events:");
    for (sequence_number, record) in &first_page.records {
        println!(
            "  #{} | tag={:?} | metadata={:?}",
            sequence_number, record.tag, record.metadata
        );
    }

    ensure!(
        first_page.records.len() == 7,
        "expected 7 DPP records (initial + product details + reward policy + distribution + commissioning + maintenance + reward payout)"
    );
    ensure!(
        on_chain.tags.tag_map.contains_key("maintenance")
            && on_chain.tags.tag_map.contains_key("recycling")
            && on_chain.tags.tag_map.contains_key("rewards"),
        "expected the DPP tag registry to contain maintenance, recycling, and rewards"
    );
    ensure!(
        on_chain.roles.roles.contains_key("Manufacturer")
            && on_chain.roles.roles.contains_key("Distributor")
            && on_chain.roles.roles.contains_key("Consumer")
            && on_chain.roles.roles.contains_key("ServiceTechnician")
            && on_chain.roles.roles.contains_key("Recycler")
            && on_chain.roles.roles.contains_key("EPRO")
            && on_chain.roles.roles.contains_key("LifecycleManager"),
        "expected all DPP roles to be registered"
    );
    ensure!(
        on_chain.updatable_metadata.as_deref() == Some("Lifecycle Stage: Maintained and Ready for Continued Use"),
        "expected the DPP lifecycle stage to reflect the completed maintenance event"
    );

    let maintenance_record = first_page
        .records
        .iter()
        .find(|(_, record)| record.metadata.as_deref() == Some("event:annual_maintenance"));
    ensure!(
        maintenance_record.is_some(),
        "expected the maintenance record to be present in the DPP history"
    );

    let reward_record = first_page
        .records
        .iter()
        .find(|(_, record)| record.metadata.as_deref() == Some("event:lcc_reward_distributed"));
    ensure!(
        reward_record.is_some(),
        "expected the reward payout record to be present in the DPP history"
    );

    println!("\nDigital Product Passport scenario completed successfully.");

    Ok(())
}

async fn issue_metadata_role(
    client: &AuditTrailClient<InMemSigner>,
    trail_id: ObjectID,
    role_name: &str,
    issued_to: IotaAddress,
) -> Result<()> {
    client
        .trail(trail_id)
        .access()
        .for_role(role_name)
        .create(PermissionSet::metadata_admin_permissions(), None)
        .build_and_execute(client)
        .await?;

    client
        .trail(trail_id)
        .access()
        .for_role(role_name)
        .issue_capability(CapabilityIssueOptions {
            issued_to: Some(issued_to),
            valid_from_ms: None,
            valid_until_ms: None,
        })
        .build_and_execute(client)
        .await?;

    Ok(())
}
