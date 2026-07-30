// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::io::Read;

#[cfg(feature = "native-grpc")]
use iota_grpc_client::Client as GrpcClient;
use iota_sdk_types::CheckpointContents;
use iota_types::{
    committee::Committee,
    effects::{TransactionEffects, TransactionEvents},
    iota_system_state::{IotaSystemStateTrait, get_iota_system_state},
    messages_checkpoint::CertifiedCheckpointSummary,
    object::Object,
    transaction::Transaction,
};
use serde::Deserialize;

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

    /// Configures verification to trust this client's source for committee data.
    ///
    /// This does not authenticate committee lineage. Use it only when the
    /// source is inside the caller's trust boundary.
    pub fn trusted_node(&self) -> CommitteeResolver<S> {
        CommitteeResolver::node(self.source.clone())
    }

    /// Configures verification to anchor at an already trusted committee.
    ///
    /// Every epoch-close checkpoint from the trusted committee up to the epoch
    /// required by each proof is authenticated before that proof is verified.
    pub fn anchored_at(&self, trusted_committee: Committee) -> CommitteeResolver<S> {
        CommitteeResolver::anchor(self.source.clone(), trusted_committee)
    }

    /// Configures verification to anchor at the committee contained in a trusted genesis blob.
    ///
    /// The reader must contain the BCS-encoded `genesis.blob` for the proof's
    /// network. The blob establishes the caller's trust anchor.
    pub fn anchored_at_genesis(
        &self,
        reader: impl Read,
    ) -> Result<CommitteeResolver<S>, Box<dyn std::error::Error + 'static>> {
        #[allow(dead_code)]
        #[derive(Deserialize)]
        struct GenesisBlob {
            checkpoint: CertifiedCheckpointSummary,
            checkpoint_contents: CheckpointContents,
            transaction: Transaction,
            effects: TransactionEffects,
            events: TransactionEvents,
            objects: Vec<Object>,
        }

        let genesis: GenesisBlob = bcs::from_reader(reader)?;
        let objects = genesis.objects.as_slice();

        let system_state = get_iota_system_state(&objects)?;
        let committee = system_state.get_current_epoch_committee().committee().clone();

        Ok(CommitteeResolver::anchor(self.source.clone(), committee))
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
