// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { Proof, ProofBuilder } from "../node/poi_wasm.js";
import type { LedgerSource } from "../lib/source-types.js";

test("the WASM builder reads transaction evidence from the ledger source", async () => {
  const transactionDigest = new Uint8Array(32).fill(0x2a);
  let requestedDigest: Uint8Array | undefined;
  const source = {
    async transaction(digest: Uint8Array) {
      requestedDigest = digest;

      return {
        // Deliberately invalid BCS: the test is proving that the WASM adapter
        // reached this source and attempted Rust-side decoding.
        transactionBcs: new Uint8Array([0xff]),
        signaturesBcs: [],
        effectsBcs: new Uint8Array([0xff]),
        checkpointSequenceNumber: 7n,
      };
    },
  } as unknown as LedgerSource;

  await assert.rejects(
    new ProofBuilder(source).transaction(transactionDigest).build(),
    /source failed while reading transaction .*: source returned an invalid response/,
  );
  assert.deepEqual(requestedDigest, transactionDigest);
});

test("the WASM builder validates digest lengths before fetching", () => {
  const source = {} as LedgerSource;

  assert.throws(
    () => new ProofBuilder(source).transaction(new Uint8Array(31)),
    /invalid digest byte length: expected 32, got 31/,
  );
});

test("the WASM proof can be deserialized for verification", async () => {
  const json = await readFile(
    new URL(
      "../../../../poi-rs/tests/fixtures/current/transaction.json",
      import.meta.url,
    ),
    "utf8",
  );

  const proof = Proof.fromJSON(json);
  const serialized = JSON.parse(proof.toJSON()) as {
    targets: {
      transaction: string | null;
      objects: unknown[];
      events: unknown[];
    };
    checkpoint_contents: unknown;
    transaction_proof: Record<string, unknown>;
  };

  assert.equal(proof.version, 1);
  assert.equal(proof.checkpointEpoch, 0n);
  assert.doesNotThrow(() => proof.validate());
  assert.equal(typeof serialized.targets.transaction, "string");
  assert.deepEqual(serialized.targets.objects, []);
  assert.deepEqual(serialized.targets.events, []);
  assert.ok(serialized.checkpoint_contents);
  assert.deepEqual(Object.keys(serialized.transaction_proof), [
    "transaction",
    "effects",
    "events",
  ]);
});

test("the WASM proof keeps selected events separate from event evidence", async () => {
  const json = await readFile(
    new URL(
      "../../../../poi-rs/tests/fixtures/current/event.json",
      import.meta.url,
    ),
    "utf8",
  );

  const proof = Proof.fromJSON(json);
  const serialized = JSON.parse(proof.toJSON()) as {
    targets: {
      transaction: string | null;
      objects: unknown[];
      events: Array<{ txDigest: string; eventSeq: string }>;
    };
    transaction_proof: { events: unknown[] | null };
  };

  assert.equal(serialized.targets.transaction, null);
  assert.deepEqual(serialized.targets.objects, []);
  assert.equal(serialized.targets.events.length, 1);
  assert.equal(serialized.targets.events[0]?.eventSeq, "0");
  assert.equal(serialized.transaction_proof.events?.length, 1);
});

test("the WASM proof stores selected objects only in its targets", async () => {
  const json = await readFile(
    new URL(
      "../../../../poi-rs/tests/fixtures/current/object.json",
      import.meta.url,
    ),
    "utf8",
  );

  const proof = Proof.fromJSON(json);
  const serialized = JSON.parse(proof.toJSON()) as {
    targets: {
      transaction: string | null;
      objects: unknown[];
      events: unknown[];
    };
    transaction_proof: { events: unknown[] | null };
  };

  assert.equal(serialized.targets.transaction, null);
  assert.equal(serialized.targets.objects.length, 1);
  assert.deepEqual(serialized.targets.events, []);
  assert.equal(serialized.transaction_proof.events, null);
});
