// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_grpc_client::Client as GrpcClient;
use iota_types::{base_types::ObjectRef, digests::TransactionDigest, event::EventID};

use crate::{Proof, Source, SourceError, SourceTarget, source::GrpcSource};

/// Error returned when a proof cannot be constructed by [`ProofBuilder`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProofBuilderError {
    /// No proof target was selected before building.
    #[error("proof builder requires a target")]
    MissingTarget,
    /// The configured source failed to construct the requested proof.
    #[error("proof source failed")]
    Source {
        /// Underlying source failure.
        #[source]
        source: SourceError,
    },
}

/// Constructs Proof of Inclusion evidence from a caller-provided [`Source`].
///
/// The builder keeps proof construction independent of a specific transport.
/// SDK gRPC clients can be adapted through [`ProofBuilder::from_grpc_client`].
pub struct ProofBuilder<S> {
    source: S,
    targets: Vec<SourceTarget>,
}

impl ProofBuilder<GrpcSource> {
    /// Creates a proof builder connected to the public IOTA mainnet gRPC endpoint.
    ///
    /// Selecting an endpoint does not establish verification trust. Verify the
    /// constructed proof with a committee trusted for mainnet.
    pub fn mainnet() -> iota_grpc_client::Result<Self> {
        GrpcClient::new_mainnet().map(Self::from_grpc_client)
    }

    /// Creates a proof builder connected to the public IOTA testnet gRPC endpoint.
    ///
    /// Selecting an endpoint does not establish verification trust. Verify the
    /// constructed proof with a committee trusted for testnet.
    pub fn testnet() -> iota_grpc_client::Result<Self> {
        GrpcClient::new_testnet().map(Self::from_grpc_client)
    }

    /// Creates a proof builder connected to the public IOTA devnet gRPC endpoint.
    ///
    /// Selecting an endpoint does not establish verification trust. Verify the
    /// constructed proof with a committee trusted for devnet.
    pub fn devnet() -> iota_grpc_client::Result<Self> {
        GrpcClient::new_devnet().map(Self::from_grpc_client)
    }

    /// Creates a proof builder backed by an existing SDK gRPC client.
    pub fn from_grpc_client(client: GrpcClient) -> Self {
        Self::new(GrpcSource::new(client))
    }
}

impl<S: Source> ProofBuilder<S> {
    /// Creates a proof builder backed by `source`.
    pub fn new(source: S) -> Self {
        Self {
            source,
            targets: Vec::new(),
        }
    }

    /// Adds a transaction proof target.
    pub fn transaction(mut self, transaction_digest: TransactionDigest) -> Self {
        self.push_target(SourceTarget::Transaction(transaction_digest));
        self
    }

    /// Adds an object proof target.
    pub fn object(mut self, object_ref: ObjectRef) -> Self {
        self.push_target(SourceTarget::Object(object_ref));
        self
    }

    /// Adds multiple object proof targets.
    pub fn objects(mut self, object_refs: impl IntoIterator<Item = ObjectRef>) -> Self {
        for object_ref in object_refs {
            self.push_target(SourceTarget::Object(object_ref));
        }
        self
    }

    /// Adds an event proof target.
    pub fn event(mut self, event_id: EventID) -> Self {
        self.push_target(SourceTarget::Event(event_id));
        self
    }

    /// Adds multiple event proof targets.
    pub fn events(mut self, event_ids: impl IntoIterator<Item = EventID>) -> Self {
        for event_id in event_ids {
            self.push_target(SourceTarget::Event(event_id));
        }
        self
    }

    /// Builds the requested proof from the configured source.
    pub async fn build(self) -> Result<Proof, ProofBuilderError> {
        if self.targets.is_empty() {
            return Err(ProofBuilderError::MissingTarget);
        }

        let proof = self
            .source
            .proof(&self.targets)
            .await
            .map_err(|source| ProofBuilderError::Source { source })?;

        Ok(proof)
    }

    fn push_target(&mut self, target: SourceTarget) {
        if !self.targets.contains(&target) {
            self.targets.push(target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mainnet_uses_the_sdk_mainnet_endpoint() {
        let builder = ProofBuilder::mainnet().expect("mainnet builder must be configured");
        let expected = GrpcClient::new_mainnet().expect("SDK mainnet client must be configured");

        assert_eq!(builder.source.grpc_client().uri(), expected.uri());
    }

    #[tokio::test]
    async fn testnet_uses_the_sdk_testnet_endpoint() {
        let builder = ProofBuilder::testnet().expect("testnet builder must be configured");
        let expected = GrpcClient::new_testnet().expect("SDK testnet client must be configured");

        assert_eq!(builder.source.grpc_client().uri(), expected.uri());
    }

    #[tokio::test]
    async fn devnet_uses_the_sdk_devnet_endpoint() {
        let builder = ProofBuilder::devnet().expect("devnet builder must be configured");
        let expected = GrpcClient::new_devnet().expect("SDK devnet client must be configured");

        assert_eq!(builder.source.grpc_client().uri(), expected.uri());
    }
}
