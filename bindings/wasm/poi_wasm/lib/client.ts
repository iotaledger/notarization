// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { type Client, createClient, type Transport } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";

import { LedgerService } from "./grpc/generated/iota/grpc/v1/ledger_service_pb.js";

const DEFAULT_TIMEOUT_MS = 30_000;
const DEFAULT_READ_MAX_BYTES = 128 * 1024 * 1024;

export type IotaGrpcClient = Client<typeof LedgerService>;

export interface IotaGrpcClientOptions {
    defaultTimeoutMs?: number;
    readMaxBytes?: number;
    transport?: Transport;
}

/**
 * Creates a Node.js gRPC client from the generated IOTA LedgerService
 * descriptor.
 */
export function createIotaGrpcClient(
    endpoint: string,
    options: IotaGrpcClientOptions = {},
): IotaGrpcClient {
    const baseUrl = endpoint.trim().replace(/\/+$/, "");

    if (!baseUrl) {
        throw new Error("IOTA gRPC endpoint must not be empty");
    }

    const transport = options.transport
        ?? createGrpcTransport({
            baseUrl,
            defaultTimeoutMs: options.defaultTimeoutMs ?? DEFAULT_TIMEOUT_MS,
            readMaxBytes: options.readMaxBytes ?? DEFAULT_READ_MAX_BYTES,
        });

    return createClient(LedgerService, transport);
}
