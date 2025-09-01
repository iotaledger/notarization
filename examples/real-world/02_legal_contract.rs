// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Legal Contract Example - Locked Notarization
//!
//! This example demonstrates how to use notarization fields for immutable legal contracts.
//! Once created, locked notarizations cannot be modified, ensuring contract integrity.
//!
//! ## Field Usage Strategy:
//!
//! - **state.data**: Contract document hash (SHA-256 of the actual contract PDF)
//! - **state.metadata**: Contract metadata (type, effective date, duration, parties)
//! - **immutable_description**: Human-readable contract description and parties
//! - **updatable_metadata**: Administrative info (filing references, storage location)
//!
//! Note: For locked notarizations, ALL fields become immutable after creation,
//! including the "updatable_metadata" field name which is historical.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use examples::get_funded_client;
use notarization::core::types::{State, TimeLock};
use serde_json::json;
use sha2::{Digest, Sha256};

#[tokio::main]
async fn main() -> Result<()> {
    println!("⚖️  Legal Contract - Locked Notarization Example");
    println!("===============================================\n");

    let notarization_client = get_funded_client().await?;

    // Get current timestamp
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    // Simulate contract creation with realistic data
    println!("📄 Creating immutable legal contract notarization...");

    // Simulate a contract document (in reality, this would be the actual PDF content)
    let contract_content = r#"
            EMPLOYMENT AGREEMENT

            This Employment Agreement is entered into on January 28, 2025, between:

            EMPLOYER: ACME Corporation
            Address: 123 Business Ave, Hamburg, Germany
            Registration: HRB 12345

            EMPLOYEE: John Doe  
            Address: 456 Residential St, Hamburg, Germany
            ID: ID123456789

            TERMS:
            - Position: Senior Software Engineer
            - Start Date: February 1, 2025  
            - Salary: €75,000 annually
            - Duration: 2 years (until January 31, 2027)
            - Probation Period: 6 months

            [Additional terms and conditions would follow...]

            Signatures:
            ACME Corporation: [Digital Signature]
            John Doe: [Digital Signature]

            Date: January 28, 2025
            "#;

    // Calculate SHA-256 hash of the contract
    let mut hasher = Sha256::new();
    hasher.update(contract_content.as_bytes());
    let contract_hash = format!("{:x}", hasher.finalize());

    // Create structured contract metadata
    let contract_metadata = json!({
        "contract_type": "Employment Agreement",
        "effective_date": "2025-02-01",
        "expiration_date": "2027-01-31",
        "duration_years": 2,
        "employer": "ACME Corporation",
        "employee": "John Doe",
        "governing_law": "German Labor Law",
        "hash_algorithm": "SHA-256",
        "document_size_bytes": contract_content.len(),
        "created_timestamp": now
    });

    // Calculate deletion unlock time (7 years for legal document retention)
    let deletion_unlock = now + (7 * 365 * 24 * 60 * 60); // 7 years from now

    println!("🔒 Contract Hash: {}", &contract_hash[..16]);
    println!("📅 Contract Effective: 2025-02-01 to 2027-01-31");
    println!(
        "🗓️  Legal Retention: 7 years (until {})",
        format_timestamp(deletion_unlock)
    );

    // Create locked notarization for legal contract
    let contract_notarization = notarization_client
        .create_locked_notarization()
        // state.data: The SHA-256 hash of the actual contract document
        // This proves document integrity - any change would result in a different hash
        .with_string_state(
            contract_hash.clone(),
            Some(contract_metadata.to_string())
        )
        // immutable_description: Human-readable contract summary for legal identification
        // This helps legal professionals quickly identify the contract without revealing sensitive details
        .with_immutable_description(
            "Employment Agreement between ACME Corporation (Employer) and John Doe (Employee) | Effective: Feb 1, 2025 - Jan 31, 2027 | Position: Senior Software Engineer".to_string()
        )
        // updatable_metadata: Administrative filing information
        // NOTE: For locked notarizations, this becomes immutable after creation!
        .with_updatable_metadata(
            format!("Filed: {} | HR Reference: HR-2025-001-EA | Legal Review: Completed | Storage: Digital Vault A7 | Notarization: {}", 
                format_timestamp(now),
                format_timestamp(now)
            )
        )
        // Delete lock: Contract can only be deleted after 7-year legal retention period
        .with_delete_lock(TimeLock::UnlockAt(deletion_unlock as u32))
        .finish()?
        .build_and_execute(&notarization_client)
        .await?;

    let notarization_id = contract_notarization.output.id.object_id();
    println!("✅ Legal contract notarization created!");
    println!("🔗 Notarization ID: {}", notarization_id);

    // Display the contract notarization details
    display_contract_details(&contract_notarization.output.clone())?;

    // Demonstrate immutability by attempting to update (this will fail)
    println!("\n🚫 Demonstrating Immutability Protection...\n");

    println!("⚠️  Attempting to update contract hash (this will fail):");
    let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";

    match notarization_client
        .update_state(
            State::from_string(fake_hash.to_string(), Some("Tampered metadata".to_string())),
            *notarization_id,
        )
        .build_and_execute(&notarization_client)
        .await
    {
        Ok(_) => println!("❌ ERROR: Update should have failed!"),
        Err(e) => {
            println!("✅ Update correctly rejected: Contract remains immutable");
            println!("🔒 Error details: {}", e);
        }
    }

    println!("\n⚠️  Attempting to update metadata (this will also fail):");
    match notarization_client
        .update_metadata(Some("Tampered administrative info".to_string()), *notarization_id)
        .build_and_execute(&notarization_client)
        .await
    {
        Ok(_) => println!("❌ ERROR: Metadata update should have failed!"),
        Err(e) => {
            println!("✅ Metadata update correctly rejected: All fields are immutable");
            println!("🔒 Error details: {}", e);
        }
    }

    // Verify the contract remains unchanged
    println!("\n✅ Verifying contract integrity...");
    let verified_notarization = notarization_client.get_notarization_by_id(*notarization_id).await?;

    let stored_hash = verified_notarization.state.data().clone().as_text()?;
    if stored_hash == contract_hash {
        println!("🎯 Contract integrity verified: Hash matches original");
    } else {
        println!("❌ CRITICAL ERROR: Contract hash has been tampered with!");
    }

    println!("\n🎯 Example Complete!");
    println!("\n💡 Key Takeaways:");
    println!("• 🔒 Locked notarizations are completely immutable after creation");
    println!("• 📄 state.data: Store document hash to ensure integrity");
    println!("• 📋 state.metadata: Include structured contract details");
    println!("• 📝 immutable_description: Human-readable contract identification");
    println!("• 📁 updatable_metadata: Administrative info (but immutable for locked!)");
    println!("• ⏰ delete_lock: Enforces legal retention periods");
    println!("\nLocked notarizations provide tamper-proof legal document attestation!");

    Ok(())
}

/// Helper function to display contract details in a structured format
fn display_contract_details(notarization: &notarization::core::types::OnChainNotarization) -> Result<()> {
    println!("\n📋 Contract Notarization Details");
    println!("─────────────────────────────────");

    // Display state.data (contract hash)
    println!("🔐 Document Hash: {}", notarization.state.data.clone().as_text()?);

    // Parse and display state.metadata (contract details)
    if let Some(metadata) = notarization.state.metadata()
        && let Ok(contract_data) = serde_json::from_str::<serde_json::Value>(metadata)
    {
        println!("📄 Contract Type: {}", contract_data["contract_type"]);
        println!("👔 Employer: {}", contract_data["employer"]);
        println!("👤 Employee: {}", contract_data["employee"]);
        println!(
            "📅 Effective: {} to {}",
            contract_data["effective_date"], contract_data["expiration_date"]
        );
        println!("⚖️  Governing Law: {}", contract_data["governing_law"]);
        println!("📊 Document Size: {} bytes", contract_data["document_size_bytes"]);
    }

    // Display immutable_description (contract summary)
    if let Some(description) = notarization.immutable_metadata.description.clone() {
        println!("📝 Description: {}", description);
    }

    // Display updatable_metadata (administrative info - but immutable for locked!)
    if let Some(admin_info) = &notarization.updatable_metadata {
        println!("📁 Administrative: {}", admin_info);
    }

    println!(
        "🕐 Created: {}",
        format_timestamp(notarization.immutable_metadata.created_at / 1000)
    );
    println!(
        "🔢 Version: {} (will never change for locked notarizations)",
        notarization.state_version_count
    );

    // Display lock information
    if let Some(lock_metadata) = notarization.immutable_metadata.locking.clone() {
        println!("🔒 Immutable: All fields locked until destruction");
        println!("Lock Metadata: {:?}", lock_metadata);
        println!("🗑️  Deletion Allowed: After legal retention period expires");
    }

    Ok(())
}

/// Helper function to format Unix timestamp as readable date
fn format_timestamp(timestamp: u64) -> String {
    use chrono::{DateTime, Utc};
    DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Invalid timestamp".to_string())
}
