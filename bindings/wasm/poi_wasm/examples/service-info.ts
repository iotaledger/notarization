// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { NodePoiSource } from "../src/index.js";

const endpoint = process.argv[2] ?? "https://grpc.testnet.iota.cafe:443";
const source = new NodePoiSource(endpoint);
const chainIdentifier = await source.chainIdentifier();

console.log({
  endpoint,
  chainIdentifier: Buffer.from(chainIdentifier).toString("hex"),
});
