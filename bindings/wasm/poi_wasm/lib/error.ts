// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/** Stable error codes returned by the Proof of Inclusion Wasm Package. */
export type PoiErrorCode =
    | "PROOF_INVALID"
    | "COMMITTEE_RESOLUTION"
    | "SOURCE_REQUEST"
    | "NOT_FOUND"
    | "INVALID_INPUT"
    | "INTERNAL";

/** Error returned by the Proof of Inclusion Wasm Package. */
export interface PoiError<Code extends PoiErrorCode = PoiErrorCode> extends Error {
    readonly code: Code;
}

/** Returns whether `error` is a Proof of Inclusion error with a stable code. */
export function isPoiError(error: unknown): error is PoiError {
    if (!(error instanceof Error)) {
        return false;
    }

    return error.name === "PoiError" && isPoiErrorCode((error as { code?: unknown }).code);
}

function isPoiErrorCode(code: unknown): code is PoiErrorCode {
    return code === "PROOF_INVALID"
        || code === "COMMITTEE_RESOLUTION"
        || code === "SOURCE_REQUEST"
        || code === "NOT_FOUND"
        || code === "INVALID_INPUT"
        || code === "INTERNAL";
}
