// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import { PoiClient } from "../lib/index.js";

test("creates clients for every supported public network", () => {
  const clients = [
    PoiClient.mainnet(),
    PoiClient.testnet(),
    PoiClient.devnet(),
  ];

  for (const client of clients) {
    assert.equal(typeof client.proof().transaction, "function");
  }
});

test("creates a client for an explicit endpoint", () => {
  const client = new PoiClient("http://localhost:9000");

  assert.equal(typeof client.proof().transaction, "function");
});
