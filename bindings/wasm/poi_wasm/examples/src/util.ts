// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { execFile } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import { KeytoolSigner } from "@iota/iota-interaction-ts/node";
import { IotaClient } from "@iota/iota-sdk/client";
import { Transaction } from "@iota/iota-sdk/transactions";
import {
    IOTA_CLOCK_OBJECT_ID,
    MOVE_STDLIB_ADDRESS,
    normalizeIotaAddress,
    normalizeIotaObjectId,
} from "@iota/iota-sdk/utils";
import { CommitteeResolution, PoiClient } from "@iota/poi-wasm";

const execFileAsync = promisify(execFile);
const REPOSITORY_ROOT = fileURLToPath(new URL("../../../../../", import.meta.url));
const PUBLISH_SCRIPT_PATH = join(REPOSITORY_ROOT, "notarization-move", "scripts", "publish_package.sh");
const PACKAGE_CACHE_DIRECTORY = join(REPOSITORY_ROOT, "target", "poi-examples");

const MAINNET_CHAIN_IDENTIFIER = "6364aad5";
const TESTNET_CHAIN_IDENTIFIER = "2304aa97";
const DEVNET_CHAIN_IDENTIFIER = "daf90477";
const NOTARIZATION_PACKAGE_ID_ENV = "IOTA_NOTARIZATION_PKG_ID";
const GRPC_URL_ENV = "NETWORK_GRPC_URL";
const GENESIS_PATH_ENV = "IOTA_GENESIS_PATH";
const TEST_GAS_BUDGET = 50_000_000n;

interface CliEnvironment {
    alias: string;
    rpc: string;
    grpc?: string | null;
}

/** Clients and network metadata shared by the TypeScript Proof of Inclusion examples. */
export interface PoiContext {
    /** Active IOTA CLI environment alias. */
    networkAlias: string;
    /** Chain identifier reported by the active network. */
    chainIdentifier: string;
    /** Client used to construct and verify Proof of Inclusion proofs. */
    poiClient: PoiClient;
    /** JSON-RPC client connected to the active IOTA CLI environment. */
    rpcClient: IotaClient;
    /** Signer backed by the active IOTA CLI wallet. */
    signer: KeytoolSigner;
    /** Address selected in the active IOTA CLI wallet. */
    senderAddress: string;
    /** Single Notarization Move Package used to create example evidence. */
    notarizationPackageId: string;
}

/** Fresh transaction, object, and event targets produced for an example. */
export interface NotarizationTargets {
    /** Digest of the transaction that created the `Notarization` object. */
    transactionDigest: string;
    /** Locked `Notarization` object created by the transaction. */
    objectId: string;
    /** Sequence number of the `LockedNotarizationCreated` event. */
    eventSequence: bigint;
}

/**
 * Configures the TypeScript examples from the active IOTA CLI environment and wallet.
 *
 * `IOTA_NOTARIZATION_PKG_ID` overrides package discovery. When it is absent on
 * a non-mainnet network, the utility publishes Single Notarization once and
 * caches its Package ID by chain identifier. Mainnet always requires an
 * explicit Package ID.
 */
export async function preparePoiExample(): Promise<PoiContext> {
    const environment = await activeCliEnvironment();
    const rpcClient = new IotaClient({ url: environment.rpc });
    const chainIdentifier = await rpcClient.getChainIdentifier();
    const grpcUrl = resolveGrpcUrl(environment, chainIdentifier);
    const poiClient = new PoiClient(grpcUrl);
    const isMainnet = environment.alias === "mainnet" || chainIdentifier === MAINNET_CHAIN_IDENTIFIER;
    const configuredPackageId = configuredPackageIdFromEnvironment();

    if (isMainnet && !configuredPackageId) {
        throw new Error(
            `${NOTARIZATION_PACKAGE_ID_ENV} must be set on mainnet; the examples never publish a Package automatically on mainnet`,
        );
    }

    const cachedPackageId = configuredPackageId ? undefined : await readCachedPackageId(chainIdentifier);
    const existingPackageId = configuredPackageId ?? cachedPackageId;
    const requiredBalance = TEST_GAS_BUDGET * 2n + (existingPackageId ? 0n : 500_000_000n);
    const senderAddress = await activeCliAddress();

    await fundWalletIfNeeded(rpcClient, senderAddress, requiredBalance, isMainnet);

    const packageId = existingPackageId ?? (await publishAndCachePackage(chainIdentifier));

    return {
        networkAlias: environment.alias,
        chainIdentifier,
        poiClient,
        rpcClient,
        signer: new KeytoolSigner(senderAddress),
        senderAddress,
        notarizationPackageId: packageId,
    };
}

/** Creates fresh transaction, object, and event targets through Single Notarization. */
export async function createNotarization(
    context: PoiContext,
    label = "Notarization object",
): Promise<NotarizationTargets> {
    const transaction = createLockedNotarizationTransaction(context.notarizationPackageId, label);
    transaction.setSender(context.senderAddress);
    transaction.setGasBudget(TEST_GAS_BUDGET);

    const transactionBytes = await transaction.build({ client: context.rpcClient });
    const signature = await context.signer.sign(transactionBytes);
    const submitted = await context.rpcClient.executeTransactionBlock({
        transactionBlock: transactionBytes,
        signature,
    });
    const finalized = await context.rpcClient.waitForTransaction({
        digest: submitted.digest,
        options: {
            showEffects: true,
            showEvents: true,
            showObjectChanges: true,
        },
    });

    if (finalized.effects?.status.status !== "success") {
        throw new Error(`setup transaction failed: ${finalized.effects?.status.error ?? "missing execution status"}`);
    }

    const event = finalized.events?.find((candidate) =>
        candidate.type.endsWith("::locked_notarization::LockedNotarizationCreated"),
    );
    if (!event) {
        throw new Error("setup transaction did not emit the expected LockedNotarizationCreated event");
    }
    if (event.id.txDigest !== finalized.digest) {
        throw new Error("setup event does not belong to the setup transaction");
    }

    const objectId = normalizeIotaObjectId(readNotarizationId(event.parsedJson), false, true);
    const objectWasCreated = finalized.objectChanges?.some(
        (change) =>
            change.type === "created" &&
            normalizeIotaObjectId(change.objectId, false, true) === objectId,
    );
    if (!objectWasCreated) {
        throw new Error("setup transaction did not create the Notarization object named by its creation event");
    }

    console.log(`  ${label}:`);
    console.log(`    network:              ${context.networkAlias}`);
    console.log(`    Notarization Package: ${context.notarizationPackageId}`);
    console.log(`    transaction:          ${finalized.digest}`);
    console.log(`    Notarization object:  ${objectId}`);
    console.log(`    creation event:       ${event.id.txDigest}:${event.id.eventSeq}`);

    return {
        transactionDigest: finalized.digest,
        objectId,
        eventSequence: BigInt(event.id.eventSeq),
    };
}

function createLockedNotarizationTransaction(packageId: string, label: string): Transaction {
    const transaction = new Transaction();
    const state = transaction.moveCall({
        target: `${packageId}::notarization::new_state_from_string`,
        arguments: [transaction.pure.string(label), transaction.pure.option("string", null)],
    });
    const deleteLock = transaction.moveCall({
        target: `${packageId}::timelock::none`,
    });

    transaction.moveCall({
        target: `${packageId}::locked_notarization::create`,
        typeArguments: [`${MOVE_STDLIB_ADDRESS}::string::String`],
        arguments: [
            state,
            transaction.pure.option("string", null),
            transaction.pure.option("string", null),
            deleteLock,
            transaction.object(IOTA_CLOCK_OBJECT_ID),
        ],
    });
    return transaction;
}

function readNotarizationId(parsedEvent: unknown): string {
    if (
        !isRecord(parsedEvent) ||
        !("notarization_id" in parsedEvent) ||
        typeof parsedEvent.notarization_id !== "string"
    ) {
        throw new Error("LockedNotarizationCreated event is missing its notarization_id");
    }
    return parsedEvent.notarization_id;
}

/** Loads a trusted genesis blob for genesis-anchored committee resolution. */
export async function loadGenesisCommitteeResolution(context: PoiContext): Promise<{
    description: string;
    resolution: CommitteeResolution;
}> {
    const configuredPath = process.env[GENESIS_PATH_ENV];
    if (configuredPath) {
        const genesis = await readFile(configuredPath);
        return {
            description: `genesis at ${configuredPath}`,
            resolution: CommitteeResolution.fromGenesis(genesis),
        };
    }

    const publicNetwork = publicNetworkForChain(context.chainIdentifier);
    if (!publicNetwork) {
        throw new Error(`set ${GENESIS_PATH_ENV} to the trusted genesis blob for the active network`);
    }

    const cachePath = join(homedir(), ".iota", "iota_config", "poi", context.chainIdentifier, "genesis.blob");
    let genesis: Uint8Array;

    try {
        genesis = await readFile(cachePath);
    } catch (error) {
        if (!isNodeError(error) || error.code !== "ENOENT") {
            throw error;
        }

        console.log(`Downloading the trusted ${publicNetwork.name} genesis blob...`);
        const response = await fetch(publicNetwork.genesisUrl);
        if (!response.ok) {
            throw new Error(
                `failed to download the ${publicNetwork.name} genesis blob: ${response.status} ${response.statusText}`,
            );
        }

        genesis = new Uint8Array(await response.arrayBuffer());
        await mkdir(dirname(cachePath), { recursive: true });
        await writeFile(cachePath, genesis);
    }

    return {
        description: `${publicNetwork.name} genesis`,
        resolution: CommitteeResolution.fromGenesis(genesis),
    };
}

/** Returns the elapsed milliseconds since a `process.hrtime.bigint()` reading. */
export function elapsedMilliseconds(start: bigint): number {
    return Number(process.hrtime.bigint() - start) / 1_000_000;
}

async function activeCliEnvironment(): Promise<CliEnvironment> {
    const stdout = await runCommand(
        "iota",
        ["client", "envs", "--json"],
        "read the configured IOTA CLI environments",
    );
    const parsed: unknown = JSON.parse(stdout);

    if (!Array.isArray(parsed) || parsed.length !== 2 || !Array.isArray(parsed[0]) || typeof parsed[1] !== "string") {
        throw new Error("`iota client envs --json` returned an unexpected response");
    }

    const activeAlias = parsed[1];
    const environment = parsed[0].find(
        (candidate): candidate is CliEnvironment =>
            isRecord(candidate) &&
            candidate.alias === activeAlias &&
            typeof candidate.rpc === "string" &&
            (candidate.grpc === undefined || candidate.grpc === null || typeof candidate.grpc === "string"),
    );

    if (!environment) {
        throw new Error(`active IOTA CLI environment '${activeAlias}' was not found`);
    }
    return environment;
}

async function activeCliAddress(): Promise<string> {
    const stdout = await runCommand(
        "iota",
        ["client", "active-address", "--json"],
        "read the active IOTA CLI wallet address",
    );
    const parsed: unknown = JSON.parse(stdout);
    if (typeof parsed !== "string") {
        throw new Error("`iota client active-address --json` returned an unexpected response");
    }
    return normalizeIotaAddress(parsed, false, true);
}

function resolveGrpcUrl(environment: CliEnvironment, chainIdentifier: string): string {
    const configuredUrl = process.env[GRPC_URL_ENV];
    if (configuredUrl) {
        return configuredUrl;
    }
    if (environment.grpc) {
        return environment.grpc;
    }

    const publicNetwork = publicNetworkForChain(chainIdentifier);
    if (publicNetwork) {
        return publicNetwork.grpcUrl;
    }
    if (environment.alias === "localnet") {
        return "http://127.0.0.1:50051";
    }

    throw new Error(
        `the active CLI environment '${environment.alias}' for chain ${chainIdentifier} has no gRPC URL; set ${GRPC_URL_ENV}`,
    );
}

function configuredPackageIdFromEnvironment(): string | undefined {
    const packageId = process.env[NOTARIZATION_PACKAGE_ID_ENV];
    return packageId ? normalizeIotaObjectId(packageId, false, true) : undefined;
}

async function readCachedPackageId(chainIdentifier: string): Promise<string | undefined> {
    const cachePath = packageCachePath(chainIdentifier);
    let value: string;

    try {
        value = await readFile(cachePath, "utf8");
    } catch (error) {
        if (isNodeError(error) && error.code === "ENOENT") {
            return undefined;
        }
        throw error;
    }

    const [packageId, cachedChainIdentifier] = value.trim().split(";");
    if (!packageId || cachedChainIdentifier !== chainIdentifier) {
        return undefined;
    }
    return normalizeIotaObjectId(packageId, false, true);
}

async function publishAndCachePackage(chainIdentifier: string): Promise<string> {
    console.log("Publishing the Single Notarization Move Package...");
    const stdout = await runCommand(
        "/bin/bash",
        [PUBLISH_SCRIPT_PATH],
        "publish the Single Notarization Move Package",
    );
    const packageIdOutput = stdout.trim().split(/\r?\n/u).at(-1);
    if (!packageIdOutput) {
        throw new Error("the Package publication script did not return a Package ID");
    }
    const packageId = normalizeIotaObjectId(packageIdOutput, false, true);
    const cachePath = packageCachePath(chainIdentifier);

    await mkdir(dirname(cachePath), { recursive: true });
    await writeFile(cachePath, `${packageId};${chainIdentifier}`);
    return packageId;
}

async function fundWalletIfNeeded(
    rpcClient: IotaClient,
    address: string,
    requiredBalance: bigint,
    isMainnet: boolean,
): Promise<void> {
    const balance = BigInt((await rpcClient.getBalance({ owner: address })).totalBalance);
    if (balance >= requiredBalance) {
        return;
    }
    if (isMainnet) {
        throw new Error(
            `the active IOTA CLI wallet ${address} has ${balance} nanos but the examples require at least ${requiredBalance}; fund the wallet before running the examples`,
        );
    }

    await runCommand(
        "iota",
        ["client", "faucet", "--address", address, "--json"],
        "fund the active IOTA CLI wallet",
    );
}

function packageCachePath(chainIdentifier: string): string {
    return join(PACKAGE_CACHE_DIRECTORY, `notarization-package-${chainIdentifier}.txt`);
}

function publicNetworkForChain(
    chainIdentifier: string,
): { name: string; grpcUrl: string; genesisUrl: string } | undefined {
    switch (chainIdentifier) {
        case MAINNET_CHAIN_IDENTIFIER:
            return {
                name: "mainnet",
                grpcUrl: "https://grpc.mainnet.iota.cafe:443",
                genesisUrl: "https://dbfiles.mainnet.iota.cafe/genesis.blob",
            };
        case TESTNET_CHAIN_IDENTIFIER:
            return {
                name: "testnet",
                grpcUrl: "https://grpc.testnet.iota.cafe:443",
                genesisUrl: "https://dbfiles.testnet.iota.cafe/genesis.blob",
            };
        case DEVNET_CHAIN_IDENTIFIER:
            return {
                name: "devnet",
                grpcUrl: "https://grpc.devnet.iota.cafe:443",
                genesisUrl: "https://dbfiles.devnet.iota.cafe/genesis.blob",
            };
        default:
            return undefined;
    }
}

async function runCommand(command: string, args: string[], purpose: string): Promise<string> {
    try {
        const { stdout } = await execFileAsync(command, args, {
            encoding: "utf8",
            maxBuffer: 10 * 1024 * 1024,
        });
        return stdout;
    } catch (error) {
        const stderr = isRecord(error) && typeof error.stderr === "string" ? error.stderr.trim() : "";
        throw new Error(`failed to ${purpose}${stderr ? `: ${stderr}` : ""}`, { cause: error });
    }
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null;
}

function isNodeError(error: unknown): error is NodeJS.ErrnoException {
    return error instanceof Error && "code" in error;
}
