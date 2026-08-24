// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import type { Transport } from "@connectrpc/connect";

import {
  type CommitteeResolution,
  CommitteeResolver,
  type Proof,
  ProofBuilder,
} from "../node/poi_wasm.js";
import { LedgerSource } from "./ledger-source.js";

const MAINNET_ENDPOINT = "https://grpc.mainnet.iota.cafe:443";
const TESTNET_ENDPOINT = "https://grpc.testnet.iota.cafe:443";
const DEVNET_ENDPOINT = "https://grpc.devnet.iota.cafe:443";

/**
 * Options for configuring the ledger connection used by a {@link PoiClient}.
 */
export interface PoiClientOptions {
  /** Default timeout applied to ledger requests, in milliseconds. */
  defaultTimeoutMs?: number;
  /** Maximum response size accepted from the ledger, in bytes. */
  readMaxBytes?: number;
  /** Custom ConnectRPC transport, primarily for advanced use and testing. */
  transport?: Transport;
}

/** An event selected for inclusion in a proof. */
export interface ProofEventRequest {
  /** Digest of the transaction that emitted the event. */
  transaction: Uint8Array;
  /** Transaction-local event sequence number. */
  sequence: bigint;
}

/** Transaction, object, and event targets selected for one proof. */
export interface ProofRequest {
  /** Transaction selected as an explicit proof target. */
  transaction?: Uint8Array;
  /** Object IDs selected as proof targets. */
  objects?: readonly Uint8Array[];
  /** Event IDs selected as proof targets. */
  events?: readonly ProofEventRequest[];
}

/**
 * Creates Proofs of Inclusion backed by an IOTA ledger endpoint.
 *
 * Use one of the named public-network constructors, or construct a client with
 * an explicit endpoint for a private node, archive, local network, or
 * alternative endpoint.
 */
export class PoiClient {
  readonly #source: LedgerSource;

  /** Creates a client connected to an explicit IOTA gRPC endpoint. */
  public constructor(endpoint: string, options: PoiClientOptions = {}) {
    this.#source = new LedgerSource(endpoint, options);
  }

  /** Creates a client connected to the public IOTA mainnet gRPC endpoint. */
  public static mainnet(options: PoiClientOptions = {}): PoiClient {
    return new PoiClient(MAINNET_ENDPOINT, options);
  }

  /** Creates a client connected to the public IOTA testnet gRPC endpoint. */
  public static testnet(options: PoiClientOptions = {}): PoiClient {
    return new PoiClient(TESTNET_ENDPOINT, options);
  }

  /** Creates a client connected to the public IOTA devnet gRPC endpoint. */
  public static devnet(options: PoiClientOptions = {}): PoiClient {
    return new PoiClient(DEVNET_ENDPOINT, options);
  }

  /** Creates one Proof of Inclusion for the selected targets. */
  public async makeProof(request: ProofRequest): Promise<Proof> {
    let builder = new ProofBuilder(this.#source);

    if (request.transaction !== undefined) {
      builder = builder.transaction(request.transaction);
    }

    for (const object of request.objects ?? []) {
      builder = builder.object(object);
    }

    for (const event of request.events ?? []) {
      builder = builder.event(event.transaction, event.sequence);
    }

    return builder.build();
  }

  /** Creates a verifier using the selected committee-resolution strategy. */
  public verifier(resolution: CommitteeResolution): CommitteeResolver {
    return new CommitteeResolver(this.#source, resolution);
  }
}
