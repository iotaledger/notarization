// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { createIotaGrpcClient } from "../lib/client.js";

test("rejects an empty endpoint", () => {
    assert.throws(() => createIotaGrpcClient("  "), /endpoint must not be empty/);
});
