// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/**
 * Serialized transaction data needed by `poi-rs` to build a proof.
 *
 * These values are opaque BCS bytes. JavaScript fetches them, while Rust
 * remains responsible for decoding and validating them.
 */
export interface TransactionEvidence {
  transactionBcs: Uint8Array;
  signaturesBcs: Uint8Array[];
  effectsBcs: Uint8Array;
  eventsBcs?: Uint8Array[];
  checkpointSequenceNumber: bigint;
}

/** Serialized checkpoint data needed by `poi-rs` to authenticate a transaction. */
export interface CheckpointEvidence {
  summaryBcs: Uint8Array;
  signatureBcs: Uint8Array;
  contentsBcs: Uint8Array;
}

/** Internal JavaScript contract consumed by the WASM proof builder. */
export interface LedgerSource {
  chainIdentifier(): Promise<Uint8Array>;
  transaction(
    digest: Uint8Array,
  ): Promise<TransactionEvidence | undefined>;
  object(objectId: Uint8Array, version?: bigint): Promise<Uint8Array | undefined>;
  checkpoint(sequenceNumber: bigint): Promise<CheckpointEvidence>;
}
