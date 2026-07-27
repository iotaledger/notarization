// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import { createIotaGrpcClient } from "../lib/client.js";

const endpoint = process.argv[2] ?? "https://grpc.testnet.iota.cafe:443";
const client = createIotaGrpcClient(endpoint);
const serviceInfo = await client.getServiceInfo({
  readMask: { paths: ["chain_id"] },
});
const chainIdentifier = serviceInfo.chainId?.digest;

if (!chainIdentifier) {
  throw new Error("getServiceInfo returned no chain identifier");
}

console.log({
  endpoint,
  chainIdentifier: Buffer.from(chainIdentifier).toString("hex"),
});
