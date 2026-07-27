// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import { createRouterTransport } from "@connectrpc/connect";

import {
  CheckpointDataSchema,
  GetObjectsResponseSchema,
  GetServiceInfoResponseSchema,
  GetTransactionsResponseSchema,
  LedgerService,
} from "../src/grpc/generated/iota/grpc/v1/ledger_service_pb.js";
import { NodePoiSource } from "../src/node-poi-source.js";

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

  const transport = createRouterTransport((router) => {
    router.service(LedgerService, {
      getServiceInfo(request) {
        assert.deepEqual(request.readMask?.paths, ["chain_id"]);

        return create(GetServiceInfoResponseSchema, {
          chainId: { digest: chainId },
        });
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
    });
  });
  const source = new NodePoiSource("http://unused.test", { transport });

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
  const source = new NodePoiSource("http://unused.test", { transport });

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
  const source = new NodePoiSource("http://unused.test", { transport });

  await assert.rejects(
    source.checkpoint(42n),
    /returned no checkpoint for sequence number 42/,
  );
});

function bytes(value: number): Uint8Array {
  return new Uint8Array(32).fill(value);
}
