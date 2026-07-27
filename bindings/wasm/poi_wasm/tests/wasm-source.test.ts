// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { ProofBuilder } from "../node/poi_wasm.js";
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
    /failed to read signed transaction/,
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
