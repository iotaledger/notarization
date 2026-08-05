// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "native-grpc")]
use iota_grpc_client::Client as GrpcClient;

use crate::{CommitteeResolution, CommitteeResolver, ProofBuilder, Source};

/// Convenient entry point for proof construction and verification backed by one ledger source.
#[derive(Clone)]
pub struct PoiClient<S> {
    source: S,
}

impl<S> PoiClient<S> {
    /// Creates a client backed by `source`.
    pub const fn new(source: S) -> Self {
        Self { source }
    }
}

impl<S> PoiClient<S>
where
    S: Source + Clone,
{
    /// Creates a fresh builder for one Proof of Inclusion.
    pub fn proof(&self) -> ProofBuilder<S> {
        ProofBuilder::new(self.source.clone())
    }

    /// Creates a verifier using the selected committee-resolution strategy.
    ///
    /// Retain the returned resolver when verifying multiple proofs so anchored
    /// resolutions can reuse their authenticated committee cache.
    pub fn verifier(&self, resolution: CommitteeResolution) -> CommitteeResolver<S> {
        CommitteeResolver::new(self.source.clone(), resolution)
    }
}

#[cfg(feature = "native-grpc")]
impl PoiClient<GrpcClient> {
    /// Creates a client connected to the public IOTA mainnet gRPC endpoint.
    pub fn mainnet() -> iota_grpc_client::Result<Self> {
        GrpcClient::new_mainnet().map(Self::new)
    }

    /// Creates a client connected to the public IOTA testnet gRPC endpoint.
    pub fn testnet() -> iota_grpc_client::Result<Self> {
        GrpcClient::new_testnet().map(Self::new)
    }

    /// Creates a client connected to the public IOTA devnet gRPC endpoint.
    pub fn devnet() -> iota_grpc_client::Result<Self> {
        GrpcClient::new_devnet().map(Self::new)
    }

    /// Creates a client backed by an existing SDK gRPC client.
    pub const fn from_grpc_client(client: GrpcClient) -> Self {
        Self::new(client)
    }

    /// Returns the underlying SDK gRPC client.
    pub const fn grpc_client(&self) -> &GrpcClient {
        &self.source
    }
}
