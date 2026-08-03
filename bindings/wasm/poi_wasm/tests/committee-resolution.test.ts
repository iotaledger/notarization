// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  Committee,
  CommitteeResolution,
  CommitteeResolver,
  Proof,
} from "../node/poi_wasm.js";
import type { LedgerSource } from "../lib/source-types.js";

test("the WASM resolver constructs a committee reported by a trusted node", async () => {
  const fixture = JSON.parse(
    await readFile(
      new URL(
        "../../../../poi-rs/tests/fixtures/current/committee.json",
        import.meta.url,
      ),
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

  const committee = await new CommitteeResolver(
    source,
    CommitteeResolution.trustedNode(),
  ).resolve(0n);

  assert.equal(committee.epoch, 0n);
});

test("the anchored verifier resolves the committee and verifies the proof", async () => {
  const [committeeJson, proofJson] = await Promise.all([
    readFile(
      new URL(
        "../../../../poi-rs/tests/fixtures/current/committee.json",
        import.meta.url,
      ),
      "utf8",
    ),
    readFile(
      new URL(
        "../../../../poi-rs/tests/fixtures/current/transaction.json",
        import.meta.url,
      ),
      "utf8",
    ),
  ]);
  const committee = Committee.fromJSON(committeeJson);
  const proof = Proof.fromJSON(proofJson);
  const source = {} as LedgerSource;

  await assert.doesNotReject(
    new CommitteeResolver(
      source,
      CommitteeResolution.anchored(committee),
    ).verify(proof),
  );
});

test("the anchored resolver returns its trusted committee without fetching it again", async () => {
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
  const committee = await new CommitteeResolver(
    source,
    CommitteeResolution.trustedNode(),
  ).resolve(0n);
  const anchored = await new CommitteeResolver(
    source,
    CommitteeResolution.anchored(committee),
  ).resolve(0n);

  assert.equal(anchored.epoch, 0n);
});

test("the anchored resolver reports a missing current epoch", async () => {
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
      return undefined;
    },
  } as unknown as LedgerSource;
  const committee = await new CommitteeResolver(
    source,
    CommitteeResolution.trustedNode(),
  ).resolve(0n);

  await assert.rejects(
    new CommitteeResolver(
      source,
      CommitteeResolution.anchored(committee),
    ).resolve(1n),
    /service information is missing the current epoch/,
  );
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
  const committee = await new CommitteeResolver(
    source,
    CommitteeResolution.trustedNode(),
  ).resolve(0n);

  await assert.rejects(
    new CommitteeResolver(
      source,
      CommitteeResolution.anchored(committee),
    ).resolve(1n),
    /failed to fetch end-of-epoch checkpoint information for epoch 0/,
  );
  assert.equal(requestedEpoch, 0n);
});
