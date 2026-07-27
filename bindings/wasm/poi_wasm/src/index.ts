// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

export {
  createIotaGrpcClient,
  type IotaGrpcClient,
  type IotaGrpcClientOptions,
} from "./client.js";
export { LedgerService } from "./grpc/generated/iota/grpc/v1/ledger_service_pb.js";
export {
  NodePoiSource,
  type CheckpointEvidence,
  type TransactionEvidence,
} from "./node-poi-source.js";
export { Proof, ProofBuilder } from "../node/poi_wasm.js";
