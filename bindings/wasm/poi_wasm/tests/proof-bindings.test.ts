// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { Committee, Proof, type ProofTargets, type VerifiedProof } from "../lib/index.js";

const fixtures: readonly {
    name: string;
    assertTargets: (targets: ProofTargets) => void;
    assertVerified: (proof: VerifiedProof) => void;
}[] = [
    {
        name: "transaction",
        assertTargets(targets) {
            assert.equal(
                targets.transaction,
                "W5a5vsCEVHTj5woXy1MymQYpe4UFzwEoor8k8PASDeq",
            );
            assert.deepEqual(targets.objects, []);
            assert.deepEqual(targets.events, []);
        },
        assertVerified(proof) {
            assert.equal(proof.targets.transaction, proof.transaction);
            assert.deepEqual(proof.targets.objects, []);
            assert.deepEqual(proof.targets.events, []);
            assert.throws(() => proof.objectBcs(0), /object target index 0 is out of bounds/);
            assert.throws(() => proof.eventContents(0), /event target index 0 is out of bounds/);
        },
    },
    {
        name: "object",
        assertTargets(targets) {
            assert.equal(targets.transaction, undefined);
            assert.equal(targets.objects.length, 1);
            assert.match(targets.objects[0]?.objectId ?? "", /^0x[0-9a-f]{64}$/);
            assert.equal(targets.objects[0]?.version, 2n);
            assert.ok(targets.objects[0]?.digest);
            assert.deepEqual(targets.events, []);
        },
        assertVerified(proof) {
            assert.equal(proof.targets.transaction, undefined);
            assert.equal(proof.targets.objects.length, 1);
            assert.match(proof.targets.objects[0]?.objectId ?? "", /^0x[0-9a-f]{64}$/);
            assert.equal(proof.targets.objects[0]?.version, 2n);
            assert.ok(proof.targets.objects[0]?.digest);
            assert.ok(proof.objectBcs(0).length > 0);
            assert.deepEqual(proof.targets.events, []);
        },
    },
    {
        name: "event",
        assertTargets(targets) {
            assert.equal(targets.transaction, undefined);
            assert.deepEqual(targets.objects, []);
            assert.equal(targets.events.length, 1);
            assert.equal(
                targets.events[0]?.transactionDigest,
                "25zdMVEMRtg7pDGqgLgL1Hf3s8sL9YGuN9UVeo3dECG6",
            );
            assert.equal(targets.events[0]?.eventSequence, 0n);
        },
        assertVerified(proof) {
            assert.equal(proof.targets.transaction, undefined);
            assert.deepEqual(proof.targets.objects, []);
            assert.equal(proof.targets.events.length, 1);
            assert.equal(
                proof.targets.events[0]?.transactionDigest,
                "25zdMVEMRtg7pDGqgLgL1Hf3s8sL9YGuN9UVeo3dECG6",
            );
            assert.equal(proof.targets.events[0]?.eventSequence, 0n);
            assert.ok(proof.eventContents(0).length > 0);
        },
    },
];

test("the public proof fixtures round trip and verify offline", async (context) => {
    const committee = Committee.fromJSON(await readFixture("committee.json"));

    for (const fixture of fixtures) {
        await context.test(fixture.name, async () => {
            const proof = Proof.fromJSON(await readFixture(`${fixture.name}.json`));
            const receivedProof = Proof.fromJSON(proof.toJSON());

            const verified = receivedProof.verify(committee);
            assert.equal(receivedProof.version, 1);
            assert.equal(receivedProof.checkpointEpoch, 0n);
            assert.equal(verified.checkpointEpoch, 0n);
            assert.ok(verified.checkpointSequenceNumber >= 0n);
            assert.ok(verified.checkpointTimestampMs > 0n);
            assert.ok(verified.transaction);
            fixture.assertTargets(receivedProof.targets);
            fixture.assertVerified(verified);
        });
    }
});

test("rejects event sequences outside the wasm32 index range", async () => {
    const committee = Committee.fromJSON(await readFixture("committee.json"));

    for (const eventSequence of [1n << 32n, (1n << 64n) - 1n]) {
        const fixture = JSON.parse(await readFixture("event.json")) as EventProofFixture;
        fixture.ProofV1.targets.events[0]!.eventSeq = eventSequence.toString();
        const proof = Proof.fromJSON(JSON.stringify(fixture));

        assert.throws(
            () => proof.verify(committee),
            new RegExp(`event sequence number ${eventSequence} is out of bounds`),
        );
    }
});

interface EventProofFixture {
    ProofV1: {
        targets: {
            events: [{ eventSeq: string }];
        };
    };
}

function readFixture(name: string): Promise<string> {
    return readFile(
        new URL(`../../../../poi-rs/tests/fixtures/current/${name}`, import.meta.url),
        "utf8",
    );
}
