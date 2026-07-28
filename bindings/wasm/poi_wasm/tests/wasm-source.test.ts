// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { CommitteeResolver, Proof, ProofBuilder } from "../node/poi_wasm.js";
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

test("the WASM resolver constructs a committee reported by a trusted node", async () => {
  const fixture = JSON.parse(
    await readFile(
      new URL("../../../../poi-rs/tests/fixtures/current/committee.json", import.meta.url),
      "utf8",
    ),
  ) as {
    epoch: number;
    voting_rights: [string, number][];
  };
  const source = {
    async committee() {
      return {
        members: fixture.voting_rights.map(([publicKey, weight]) => ({
          publicKey: Buffer.from(publicKey, "base64"),
          weight: BigInt(weight),
        })),
      };
    },
  } as unknown as LedgerSource;

  const committee = await new CommitteeResolver(source).resolve(0n);

  assert.equal(committee.epoch, 0n);
});

test("the WASM proof can be deserialized for verification", async () => {
  const json = await readFile(
    new URL("../../../../poi-rs/tests/fixtures/current/transaction.json", import.meta.url),
    "utf8",
  );

  const proof = Proof.fromJSON(json);

  assert.equal(proof.version, 1);
  assert.equal(proof.checkpointEpoch, 0n);
  assert.doesNotThrow(() => proof.validate());
});

test("the anchored resolver returns its trusted committee without fetching it again", async () => {
  const fixture = JSON.parse(
    await readFile(
      new URL("../../../../poi-rs/tests/fixtures/current/committee.json", import.meta.url),
      "utf8",
    ),
  ) as {
    voting_rights: [string, number][];
  };
  const source = {
    async committee() {
      return {
        members: fixture.voting_rights.map(([publicKey, weight]) => ({
          publicKey: Buffer.from(publicKey, "base64"),
          weight: BigInt(weight),
        })),
      };
    },
  } as unknown as LedgerSource;
  const committee = await CommitteeResolver.node(source).resolve(0n);
  const anchored = await CommitteeResolver.anchor(source, committee).resolve(0n);

  assert.equal(anchored.epoch, 0n);
});

test("the anchored resolver requests epoch-close evidence through the JavaScript source", async () => {
  const fixture = JSON.parse(
    await readFile(
      new URL(
        "../../../../poi-rs/tests/fixtures/current/committee.json",
        import.meta.url,
      ),
      "utf8",
    ),
  ) as {
    voting_rights: [string, number][];
  };
  let requestedEpoch: bigint | undefined;
  const source = {
    async committee() {
      return {
        members: fixture.voting_rights.map(([publicKey, weight]) => ({
          publicKey: Buffer.from(publicKey, "base64"),
          weight: BigInt(weight),
        })),
      };
    },
    async currentEpoch() {
      return 1n;
    },
    async epochCloseSummary(epoch: bigint) {
      requestedEpoch = epoch;

      return {
        // Deliberately invalid BCS: the Rust adapter must receive and decode
        // the epoch-close evidence before committee authentication begins.
        summaryBcs: new Uint8Array([0xff]),
        signatureBcs: new Uint8Array([0xff]),
      };
    },
  } as unknown as LedgerSource;
  const committee = await CommitteeResolver.node(source).resolve(0n);

  await assert.rejects(
    CommitteeResolver.anchor(source, committee).resolve(1n),
    /failed to fetch end-of-epoch checkpoint information for epoch 0/,
  );
  assert.equal(requestedEpoch, 0n);
});
