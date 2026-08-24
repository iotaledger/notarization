// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import { createRouterTransport } from "@connectrpc/connect";

import { createIotaGrpcClient } from "../lib/client.js";
import {
    CheckpointDataSchema,
    GetObjectsResponseSchema,
    GetServiceInfoResponseSchema,
    GetTransactionsResponseSchema,
    LedgerService,
} from "../lib/grpc/generated/iota/grpc/v1/ledger_service_pb.js";

test("creates the generated LedgerService client", async () => {
    const requests = {
        serviceInfo: false,
        objects: false,
        transactions: false,
        checkpoint: false,
    };
    const transport = createRouterTransport((router) => {
        router.service(LedgerService, {
            getServiceInfo(request) {
                requests.serviceInfo = true;
                assert.deepEqual(request.readMask?.paths, ["chain_id"]);

                return create(GetServiceInfoResponseSchema, {
                    chainId: { digest: new Uint8Array(32).fill(0xab) },
                });
            },
            async *getObjects(request) {
                requests.objects = true;
                assert.deepEqual(request.readMask?.paths, ["bcs"]);

                yield create(GetObjectsResponseSchema, {
                    objects: [],
                    hasNext: false,
                });
            },
            async *getTransactions(request) {
                requests.transactions = true;
                assert.deepEqual(request.readMask?.paths, ["transaction.bcs"]);

                yield create(GetTransactionsResponseSchema, {
                    transactionResults: [],
                    hasNext: false,
                });
            },
            async *getCheckpoint(request) {
                requests.checkpoint = true;
                assert.deepEqual(request.checkpointId, {
                    case: "sequenceNumber",
                    value: 42n,
                });

                yield create(CheckpointDataSchema, {
                    payload: {
                        case: "endMarker",
                        value: { sequenceNumber: 42n },
                    },
                });
            },
        });
    });
    const client = createIotaGrpcClient("http://unused.test/", { transport });

    const serviceInfo = await client.getServiceInfo({
        readMask: { paths: ["chain_id"] },
    });
    const objects = await collect(
        client.getObjects({
            readMask: { paths: ["bcs"] },
        }),
    );
    const transactions = await collect(
        client.getTransactions({
            readMask: { paths: ["transaction.bcs"] },
        }),
    );
    const checkpoint = await collect(
        client.getCheckpoint({
            checkpointId: {
                case: "sequenceNumber",
                value: 42n,
            },
        }),
    );

    assert.deepEqual(serviceInfo.chainId?.digest, new Uint8Array(32).fill(0xab));
    assert.equal(objects.length, 1);
    assert.equal(transactions.length, 1);
    assert.equal(checkpoint[0]?.payload.case, "endMarker");
    assert.deepEqual(requests, {
        serviceInfo: true,
        objects: true,
        transactions: true,
        checkpoint: true,
    });
});

test("rejects an empty endpoint", () => {
    assert.throws(() => createIotaGrpcClient("  "), /endpoint must not be empty/);
});

async function collect<T>(stream: AsyncIterable<T>): Promise<T[]> {
    const values = [];

    for await (const value of stream) {
        values.push(value);
    }

    return values;
}
