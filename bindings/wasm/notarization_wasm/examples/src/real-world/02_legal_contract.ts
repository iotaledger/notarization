// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * # Legal Contract Example - Locked Notarization
 *
 * This example demonstrates how to use notarization fields for immutable legal contracts.
 * Once created, locked notarizations cannot be modified, ensuring contract integrity.
 *
 * ## Field Usage Strategy:
 *
 * - **state.data**: Contract document hash (SHA-256 of the actual contract PDF)
 * - **state.metadata**: Contract metadata (type, effective date, duration, parties)
 * - **immutable_description**: Human-readable contract description and parties
 * - **updatable_metadata**: Administrative info (filing references, storage location)
 *
 * Note: For locked notarizations, ALL fields become immutable after creation,
 * including the "updatable_metadata" field name which is historical.
 */

import { OnChainNotarization, State, TimeLock } from "@iota/notarization/node";
import { createHash } from "crypto";
import { getFundedClient } from "../util";

interface ContractMetadata {
    contract_type: string;
    effective_date: string;
    expiration_date: string;
    duration_years: number;
    employer: string;
    employee: string;
    governing_law: string;
    hash_algorithm: string;
    document_size_bytes: number;
    created_timestamp: number;
}

/** Demonstrate Legal Contract using Locked Notarization */
export async function legalContract(): Promise<void> {
    console.log("⚖️  Legal Contract - Locked Notarization Example");
    console.log("===============================================\n");

    const notarizationClient = await getFundedClient();

    // Get current timestamp
    const now = Math.floor(Date.now() / 1000);

    // Simulate contract creation with realistic data
    console.log("📄 Creating immutable legal contract notarization...");

    // Simulate a contract document (in reality, this would be the actual PDF content)
    const contractContent = `
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
`;

    // Calculate SHA-256 hash of the contract
    const contractHash = createHash("sha256").update(contractContent).digest("hex");

    // Create structured contract metadata
    const contractMetadata: ContractMetadata = {
        contract_type: "Employment Agreement",
        effective_date: "2025-02-01",
        expiration_date: "2027-01-31",
        duration_years: 2,
        employer: "ACME Corporation",
        employee: "John Doe",
        governing_law: "German Labor Law",
        hash_algorithm: "SHA-256",
        document_size_bytes: contractContent.length,
        created_timestamp: now,
    };

    // Calculate deletion unlock time (7 years for legal document retention)
    const deletionUnlock = now + (7 * 365 * 24 * 60 * 60); // 7 years from now

    console.log(`🔒 Contract Hash: ${contractHash.substring(0, 16)}...`);
    console.log("📅 Contract Effective: 2025-02-01 to 2027-01-31");
    console.log(`🗓️  Legal Retention: 7 years (until ${formatTimestamp(deletionUnlock)})`);

    // Create locked notarization for legal contract
    const contractNotarization = await notarizationClient
        .createLocked()
        // state.data: The SHA-256 hash of the actual contract document
        // This proves document integrity - any change would result in a different hash
        .withStringState(
            contractHash,
            JSON.stringify(contractMetadata),
        )
        // immutable_description: Human-readable contract summary for legal identification
        // This helps legal professionals quickly identify the contract without revealing sensitive details
        .withImmutableDescription(
            "Employment Agreement between ACME Corporation (Employer) and John Doe (Employee) | Effective: Feb 1, 2025 - Jan 31, 2027 | Position: Senior Software Engineer",
        )
        // updatable_metadata: Administrative filing information
        // NOTE: For locked notarizations, this becomes immutable after creation!
        .withUpdatableMetadata(
            `Filed: ${
                formatTimestamp(now)
            } | HR Reference: HR-2025-001-EA | Legal Review: Completed | Storage: Digital Vault A7 | Notarization: ${
                formatTimestamp(now)
            }`,
        )
        // Delete lock: Contract can only be deleted after 7-year legal retention period
        .withDeleteLock(TimeLock.withUnlockAt(deletionUnlock))
        .finish()
        .buildAndExecute(notarizationClient);

    console.log("✅ Legal contract notarization created!");
    console.log(`🔗 Notarization ID: ${contractNotarization.output.id}`);

    // Display the contract notarization details
    displayContractDetails(contractNotarization.output);

    // Demonstrate immutability by attempting to update (this will fail)
    console.log("\n🚫 Demonstrating Immutability Protection...\n");

    console.log("⚠️  Attempting to update contract hash (this will fail):");
    const fakeHash = "0000000000000000000000000000000000000000000000000000000000000000";

    try {
        await notarizationClient
            .updateState(
                State.fromString(fakeHash, "Tampered metadata"),
                contractNotarization.output.id,
            )
            .buildAndExecute(notarizationClient);
        console.log("❌ ERROR: Update should have failed!");
    } catch (error) {
        console.log("✅ Update correctly rejected: Contract remains immutable");
        console.log(`🔒 Error details: ${error}`);
    }

    console.log("\n⚠️  Attempting to update metadata (this will also fail):");
    try {
        await notarizationClient
            .updateMetadata(
                "Tampered administrative info",
                contractNotarization.output.id,
            )
            .buildAndExecute(notarizationClient);
        console.log("❌ ERROR: Metadata update should have failed!");
    } catch (error) {
        console.log("✅ Metadata update correctly rejected: All fields are immutable");
        console.log(`🔒 Error details: ${error}`);
    }

    // Verify the contract remains unchanged
    console.log("\n✅ Verifying contract integrity...");
    const verifiedNotarization = await notarizationClient.readOnly().getNotarizationById(
        contractNotarization.output.id,
    );

    const storedHash = verifiedNotarization.state.data.toString();
    if (storedHash === contractHash) {
        console.log("🎯 Contract integrity verified: Hash matches original");
    } else {
        console.log("❌ CRITICAL ERROR: Contract hash has been tampered with!");
    }

    console.log("\n🎯 Example Complete!");
    console.log("\n💡 Key Takeaways:");
    console.log("• 🔒 Locked notarizations are completely immutable after creation");
    console.log("• 📄 state.data: Store document hash to ensure integrity");
    console.log("• 📋 state.metadata: Include structured contract details");
    console.log("• 📝 immutable_description: Human-readable contract identification");
    console.log("• 📁 updatable_metadata: Administrative info (but immutable for locked!)");
    console.log("• ⏰ delete_lock: Enforces legal retention periods");
    console.log("\nLocked notarizations provide tamper-proof legal document attestation!");
}

/** Helper function to display contract details in a structured format */
function displayContractDetails(notarization: OnChainNotarization): void {
    console.log("\n📋 Contract Notarization Details");
    console.log("─────────────────────────────────");

    try {
        // Display state.data (contract hash)
        console.log(`🔐 Document Hash: ${notarization.state.data.toString()}`);

        // Parse and display state.metadata (contract details)
        if (notarization.state.metadata) {
            const contractData = JSON.parse(notarization.state.metadata);
            console.log(`📄 Contract Type: ${contractData.contract_type}`);
            console.log(`👔 Employer: ${contractData.employer}`);
            console.log(`👤 Employee: ${contractData.employee}`);
            console.log(`📅 Effective: ${contractData.effective_date} to ${contractData.expiration_date}`);
            console.log(`⚖️  Governing Law: ${contractData.governing_law}`);
            console.log(`📊 Document Size: ${contractData.document_size_bytes} bytes`);
        }

        // Display immutable_description (contract summary)
        if (notarization.immutableMetadata.description) {
            console.log(`📝 Description: ${notarization.immutableMetadata.description}`);
        }

        // Display updatable_metadata (administrative info - but immutable for locked!)
        if (notarization.updatableMetadata) {
            console.log(`📁 Administrative: ${notarization.updatableMetadata}`);
        }

        console.log(
            `🕐 Created: ${formatTimestamp(Math.floor(Number(notarization.immutableMetadata.createdAt) / 1000))}`,
        );
        console.log(`🔢 Version: ${notarization.stateVersionCount} (will never change for locked notarizations)`);

        // Display lock information
        if (notarization.immutableMetadata.locking) {
            console.log("🔒 Immutable: All fields locked until destruction");
            console.log("🗑️  Deletion Allowed: After legal retention period expires");
        }
    } catch (error) {
        console.error("Error displaying contract details:", error);
    }
}

/** Helper function to format Unix timestamp as readable date */
function formatTimestamp(timestamp: number): string {
    return new Date(timestamp * 1000).toISOString().replace("T", " ").replace(/\.\d{3}Z$/, " UTC");
}
