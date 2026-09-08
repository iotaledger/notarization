// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

export {
    Committee,
    CommitteeResolution,
    CommitteeResolver,
    Proof,
    ProofEventTarget,
    ProofObjectTarget,
    ProofTargets,
    VerifiedProof,
} from "../node/poi_wasm.js";
export { isPoiError, type PoiError, type PoiErrorCode } from "./error.js";
export { PoiClient, type PoiClientOptions, type ProofEventRequest, type ProofRequest } from "./poi-client.js";
