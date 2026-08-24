// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import type { LedgerSource } from "../lib/source-types.js";
import { Proof, ProofBuilder, type ProofTargets } from "../node/poi_wasm.js";

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
        /source failed while reading proof evidence: source returned an invalid response/,
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
    const targets: ProofTargets = proof.targets;
    const serialized = JSON.parse(proof.toJSON()) as {
        ProofV1: {
            checkpoint_contents: unknown;
            transaction_proof: Record<string, unknown>;
        };
    };

    assert.equal(proof.version, 1);
    assert.equal(proof.checkpointEpoch, 0n);
    assert.equal(
        targets.transaction,
        "W5a5vsCEVHTj5woXy1MymQYpe4UFzwEoor8k8PASDeq",
    );
    assert.deepEqual(targets.objects, []);
    assert.deepEqual(targets.events, []);
    assert.ok(serialized.ProofV1.checkpoint_contents);
    assert.deepEqual(Object.keys(serialized.ProofV1.transaction_proof), [
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
    const targets = proof.targets;
    const serialized = JSON.parse(proof.toJSON()) as {
        ProofV1: { transaction_proof: { events: unknown[] | null } };
    };

    assert.equal(targets.transaction, undefined);
    assert.deepEqual(targets.objects, []);
    assert.equal(targets.events.length, 1);
    assert.equal(
        targets.events[0]?.transactionDigest,
        "25zdMVEMRtg7pDGqgLgL1Hf3s8sL9YGuN9UVeo3dECG6",
    );
    assert.equal(targets.events[0]?.eventSequence, 0n);
    assert.equal(serialized.ProofV1.transaction_proof.events?.length, 1);
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
    const targets = proof.targets;
    const serialized = JSON.parse(proof.toJSON()) as {
        ProofV1: { transaction_proof: { events: unknown[] | null } };
    };

    assert.equal(targets.transaction, undefined);
    assert.equal(targets.objects.length, 1);
    assert.match(targets.objects[0]?.objectId ?? "", /^0x[0-9a-f]{64}$/);
    assert.equal(targets.objects[0]?.version, 2n);
    assert.ok(targets.objects[0]?.digest);
    assert.deepEqual(targets.events, []);
    assert.equal(serialized.ProofV1.transaction_proof.events, null);
});
