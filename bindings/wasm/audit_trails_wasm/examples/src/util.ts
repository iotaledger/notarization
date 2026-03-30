// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import {
    AuditTrailClient,
    AuditTrailClientReadOnly,
    CapabilityIssueOptions,
    LockingConfig,
    LockingWindow,
    PackageOverrides,
    Permission,
    PermissionSet,
    TimeLock,
} from "@iota/audit-trails/node";
import { Ed25519KeypairSigner } from "@iota/iota-interaction-ts/node/test_utils";
import { IotaClient } from "@iota/iota-sdk/client";
import { getFaucetHost, requestIotaFromFaucetV0 } from "@iota/iota-sdk/faucet";
import { Ed25519Keypair } from "@iota/iota-sdk/keypairs/ed25519";

export const IOTA_AUDIT_TRAIL_PKG_ID = globalThis?.process?.env?.IOTA_AUDIT_TRAIL_PKG_ID || "";
export const IOTA_TF_COMPONENTS_PKG_ID = globalThis?.process?.env?.IOTA_TF_COMPONENTS_PKG_ID || "";
export const NETWORK_NAME_FAUCET = globalThis?.process?.env?.NETWORK_NAME_FAUCET || "localnet";
export const NETWORK_URL = globalThis?.process?.env?.NETWORK_URL || "http://127.0.0.1:9000";
export const TEST_GAS_BUDGET = BigInt(50_000_000);

if (!IOTA_AUDIT_TRAIL_PKG_ID || !IOTA_TF_COMPONENTS_PKG_ID) {
    throw new Error(
        "IOTA_AUDIT_TRAIL_PKG_ID and IOTA_TF_COMPONENTS_PKG_ID env variables must be set to run the examples",
    );
}

export async function requestFunds(address: string) {
    await requestIotaFromFaucetV0({
        host: getFaucetHost(NETWORK_NAME_FAUCET),
        recipient: address,
    });
}

export async function getReadOnlyClient(): Promise<AuditTrailClientReadOnly> {
    const iotaClient = new IotaClient({ url: NETWORK_URL });
    return AuditTrailClientReadOnly.createWithPackageOverrides(
        iotaClient,
        new PackageOverrides(IOTA_AUDIT_TRAIL_PKG_ID, IOTA_TF_COMPONENTS_PKG_ID),
    );
}

export async function getFundedClient(): Promise<AuditTrailClient> {
    const readOnlyClient = await getReadOnlyClient();
    const keypair = Ed25519Keypair.generate();
    const signer = new Ed25519KeypairSigner(keypair);
    const client = await AuditTrailClient.create(readOnlyClient, signer);

    await requestFunds(client.senderAddress());

    const balance = await client.iotaClient().getBalance({ owner: client.senderAddress() });
    if (balance.totalBalance === "0") {
        throw new Error("Balance is still 0 after faucet funding");
    }

    console.log(`Received gas from faucet: ${balance.totalBalance} for owner ${client.senderAddress()}`);
    return client;
}

export function defaultLockingConfig(): LockingConfig {
    return new LockingConfig(
        LockingWindow.withCountBased(BigInt(100)),
        TimeLock.withNone(),
        TimeLock.withNone(),
    );
}

export async function createTrailWithSeedRecord(client: AuditTrailClient) {
    return client
        .createTrail()
        .withTrailMetadata("Example Audit Trail", "WASM example trail")
        .withUpdatableMetadata("seed metadata")
        .withLockingConfig(defaultLockingConfig())
        .withInitialRecordString("seed record", "v1")
        .finish()
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
}

export async function grantSelfRecordPermissions(client: AuditTrailClient, trailId: string): Promise<void> {
    const role = client.trail(trailId).access().forRole("example-record-writer");
    const permissions = new PermissionSet([
        Permission.AddRecord,
        Permission.DeleteRecord,
        Permission.DeleteAllRecords,
        Permission.CorrectRecord,
    ]);

    await role.create(permissions).withGasBudget(TEST_GAS_BUDGET).buildAndExecute(client);
    await role
        .issueCapability(new CapabilityIssueOptions(client.senderAddress()))
        .withGasBudget(TEST_GAS_BUDGET)
        .buildAndExecute(client);
}
