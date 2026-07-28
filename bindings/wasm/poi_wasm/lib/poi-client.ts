// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

import type { Transport } from "@connectrpc/connect";

import {
  type Committee,
  CommitteeResolver,
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

/**
 * Creates Proof of Inclusion builders backed by an IOTA ledger endpoint.
 *
 * Use one of the named public-network constructors, or {@link PoiClient.custom}
 * for a private node, archive, local network, or alternative endpoint.
 */
export class PoiClient {
  readonly #source: LedgerSource;

  private constructor(endpoint: string, options: PoiClientOptions = {}) {
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

  /** Creates a fresh builder for one Proof of Inclusion. */
  public proof(): ProofBuilder {
    return new ProofBuilder(this.#source);
  }

  /**
   * Creates a resolver that trusts this client's node for committee data.
   *
   * The resolver does not authenticate committee lineage from genesis.
   */
  public committeeResolver(): CommitteeResolver {
    return new CommitteeResolver(this.#source);
  }

  /**
   * Creates a resolver anchored at an already trusted committee.
   *
   * The resolver authenticates each epoch-close checkpoint before accepting
   * and caching the next committee.
   */
  public anchoredCommitteeResolver(
    committee: Committee,
  ): CommitteeResolver {
    return CommitteeResolver.anchor(this.#source, committee);
  }
}
