// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { Committee, Proof, type ProofTargets } from "../lib/index.js";

const fixtures: readonly {
    name: string;
    assertTargets: (targets: ProofTargets) => void;
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
    },
];

test("the public proof fixtures round trip and verify offline", async (context) => {
    const committee = Committee.fromJSON(await readFixture("committee.json"));

    for (const fixture of fixtures) {
        await context.test(fixture.name, async () => {
            const proof = Proof.fromJSON(await readFixture(`${fixture.name}.json`));
            const receivedProof = Proof.fromJSON(proof.toJSON());

            receivedProof.verify(committee);
            assert.equal(receivedProof.version, 1);
            assert.equal(receivedProof.checkpointEpoch, 0n);
            fixture.assertTargets(receivedProof.targets);
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
