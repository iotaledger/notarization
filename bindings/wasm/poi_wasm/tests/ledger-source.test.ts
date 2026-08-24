// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import { createRouterTransport } from "@connectrpc/connect";

import {
    CheckpointDataSchema,
    GetEpochResponseSchema,
    GetObjectsResponseSchema,
    GetServiceInfoResponseSchema,
    GetTransactionsResponseSchema,
    LedgerService,
} from "../lib/grpc/generated/iota/grpc/v1/ledger_service_pb.js";
import { LedgerSource } from "../lib/ledger-source.js";

test("returns the BCS evidence needed by poi-rs", async () => {
    const chainId = bytes(0x01);
    const transactionDigest = bytes(0x02);
    const transactionBcs = bytes(0x03);
    const signatureBcs = bytes(0x04);
    const effectsBcs = bytes(0x05);
    const eventBcs = bytes(0x06);
    const objectId = bytes(0x07);
    const objectBcs = bytes(0x08);
    const summaryBcs = bytes(0x09);
    const checkpointSignatureBcs = bytes(0x0a);
    const contentsBcs = bytes(0x0b);
    const committeePublicKey = new Uint8Array(96).fill(0x0c);
    const epochCloseSummaryBcs = bytes(0x0d);
    const epochCloseSignatureBcs = bytes(0x0e);
    let serviceInfoRequest = 0;
    let epochRequest = 0;

    const transport = createRouterTransport((router) => {
        router.service(LedgerService, {
            getServiceInfo(request) {
                serviceInfoRequest += 1;

                if (serviceInfoRequest === 1) {
                    assert.deepEqual(request.readMask?.paths, ["chain_id"]);

                    return create(GetServiceInfoResponseSchema, {
                        chainId: { digest: chainId },
                    });
                }

                assert.deepEqual(request.readMask?.paths, ["epoch"]);

                return create(GetServiceInfoResponseSchema, { epoch: 9n });
            },
            async *getTransactions(request) {
                assert.deepEqual(request.readMask?.paths, [
                    "transaction.bcs",
                    "signatures",
                    "effects.bcs",
                    "events.digest",
                    "events.events.bcs",
                    "checkpoint",
                ]);
                assert.deepEqual(
                    request.requests?.requests[0]?.digest?.digest,
                    transactionDigest,
                );

                yield create(GetTransactionsResponseSchema, {
                    transactionResults: [
                        {
                            result: {
                                case: "executedTransaction",
                                value: {
                                    transaction: { bcs: { data: transactionBcs } },
                                    signatures: {
                                        signatures: [{ bcs: { data: signatureBcs } }],
                                    },
                                    effects: { bcs: { data: effectsBcs } },
                                    events: {
                                        events: {
                                            events: [{ bcs: { data: eventBcs } }],
                                        },
                                    },
                                    checkpoint: 42n,
                                },
                            },
                        },
                    ],
                });
            },
            async *getObjects(request) {
                assert.deepEqual(request.readMask?.paths, ["bcs"]);
                assert.deepEqual(
                    request.requests?.requests[0]?.objectRef?.objectId?.objectId,
                    objectId,
                );
                assert.equal(
                    request.requests?.requests[0]?.objectRef?.version,
                    7n,
                );

                yield create(GetObjectsResponseSchema, {
                    objects: [
                        {
                            result: {
                                case: "object",
                                value: { bcs: { data: objectBcs } },
                            },
                        },
                    ],
                });
            },
            async *getCheckpoint(request) {
                assert.deepEqual(request.checkpointId, {
                    case: "sequenceNumber",
                    value: 42n,
                });
                assert.deepEqual(request.readMask?.paths, [
                    "checkpoint.summary.bcs",
                    "checkpoint.signature",
                    "checkpoint.contents.bcs",
                ]);

                yield create(CheckpointDataSchema, {
                    payload: {
                        case: "checkpoint",
                        value: {
                            sequenceNumber: 42n,
                            summary: { bcs: { data: summaryBcs } },
                            signature: { bcs: { data: checkpointSignatureBcs } },
                            contents: { bcs: { data: contentsBcs } },
                        },
                    },
                });
                yield create(CheckpointDataSchema, {
                    payload: {
                        case: "endMarker",
                        value: { sequenceNumber: 42n },
                    },
                });
            },
            getEpoch(request) {
                epochRequest += 1;
                assert.equal(request.epoch, 7n);

                if (epochRequest === 1) {
                    assert.deepEqual(request.readMask?.paths, ["committee"]);

                    return create(GetEpochResponseSchema, {
                        epoch: {
                            committee: {
                                epoch: 7n,
                                members: {
                                    members: [{ publicKey: committeePublicKey, weight: 10_000n }],
                                },
                            },
                        },
                    });
                }

                assert.deepEqual(request.readMask?.paths, [
                    "epoch_close_proof.checkpoint",
                ]);

                return create(GetEpochResponseSchema, {
                    epoch: {
                        epochCloseProof: {
                            checkpoint: {
                                summary: { bcs: { data: epochCloseSummaryBcs } },
                                signature: { bcs: { data: epochCloseSignatureBcs } },
                            },
                        },
                    },
                });
            },
        });
    });
    const source = new LedgerSource("http://unused.test", { transport });

    assert.deepEqual(await source.chainIdentifier(), chainId);
    assert.deepEqual(await source.transaction(transactionDigest), {
        transactionBcs,
        signaturesBcs: [signatureBcs],
        effectsBcs,
        eventsBcs: [eventBcs],
        checkpointSequenceNumber: 42n,
    });
    assert.deepEqual(await source.object(objectId, 7n), objectBcs);
    assert.deepEqual(await source.checkpoint(42n), {
        summaryBcs,
        signatureBcs: checkpointSignatureBcs,
        contentsBcs,
    });
    assert.deepEqual(await source.committee(7n), {
        members: [{ publicKey: committeePublicKey, weight: 10_000n }],
    });
    assert.equal(await source.currentEpoch(), 9n);
    assert.deepEqual(await source.epochCloseSummary(7n), {
        summaryBcs: epochCloseSummaryBcs,
        signatureBcs: epochCloseSignatureBcs,
    });
});

test("returns undefined when a transaction or object is not returned", async () => {
    const transport = createRouterTransport((router) => {
        router.service(LedgerService, {
            async *getTransactions() {
                yield create(GetTransactionsResponseSchema);
            },
            async *getObjects() {
                yield create(GetObjectsResponseSchema);
            },
        });
    });
    const source = new LedgerSource("http://unused.test", { transport });

    assert.equal(await source.transaction(bytes(0x01)), undefined);
    assert.equal(await source.object(bytes(0x02)), undefined);
});

test("rejects incomplete checkpoint evidence", async () => {
    const transport = createRouterTransport((router) => {
        router.service(LedgerService, {
            async *getCheckpoint() {
                yield create(CheckpointDataSchema, {
                    payload: {
                        case: "endMarker",
                        value: { sequenceNumber: 42n },
                    },
                });
            },
        });
    });
    const source = new LedgerSource("http://unused.test", { transport });

    await assert.rejects(
        source.checkpoint(42n),
        /returned no checkpoint for sequence number 42/,
    );
});

function bytes(value: number): Uint8Array {
    return new Uint8Array(32).fill(value);
}
