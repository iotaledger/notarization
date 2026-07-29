// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "native-grpc")]
use iota_grpc_client::Client as GrpcClient;
use iota_types::committee::Committee;

use crate::{CommitteeResolver, ProofBuilder, Source};

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

    /// Creates a verifier that trusts this client's source for committee data.
    ///
    /// This mode does not authenticate committee lineage. Use it only when the
    /// source is inside the caller's trust boundary.
    pub fn trusted_node_verifier(&self) -> CommitteeResolver<S> {
        CommitteeResolver::node(self.source.clone())
    }

    /// Creates a verifier anchored at an already trusted committee.
    ///
    /// The verifier authenticates every epoch-close checkpoint from the trusted
    /// committee up to the epoch required by each proof.
    pub fn anchored_verifier(&self, trusted_committee: Committee) -> CommitteeResolver<S> {
        CommitteeResolver::anchor(self.source.clone(), trusted_committee)
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
