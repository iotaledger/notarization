// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { Committee } from "../lib/index.js";

test("the WASM committee can be deserialized from Rust JSON", async () => {
    const json = await readFile(
        new URL(
            "../../../../poi-rs/tests/fixtures/current/committee.json",
            import.meta.url,
        ),
        "utf8",
    );

    const committee = Committee.fromJSON(json);
    const restored = Committee.fromJSON(committee.toJSON());

    assert.equal(committee.epoch, 0n);
    assert.equal(restored.epoch, committee.epoch);
    assert.equal(restored.toJSON(), committee.toJSON());
});

test("the WASM committee rejects invalid total voting power", async () => {
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
    fixture.voting_rights[0]![1] = 9_999;

    assert.throws(
        () => Committee.fromJSON(JSON.stringify(fixture)),
        /committee voting power must total 10000, received 9999/,
    );
});
