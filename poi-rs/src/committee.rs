// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

#[cfg(feature = "native-grpc")]
use iota_grpc_client::Client as GrpcClient;
use iota_types::{
    committee::{Committee, EpochId},
    error::IotaError,
    messages_checkpoint::CertifiedCheckpointSummary,
};

use crate::{
    BoxError, CommitteeCache, CommitteeCacheError, MemoryCommitteeCache, Proof, ProofVerifier, Source, VerifyError,
};

/// Error returned when a committee cannot be resolved for an epoch.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[error("failed to resolve committee for epoch {target_epoch}")]
pub struct CommitteeResolutionError {
    /// Epoch whose committee was requested.
    pub target_epoch: EpochId,
    /// Committee resolution failure details.
    #[source]
    pub kind: CommitteeResolutionErrorKind,
}

impl CommitteeResolutionError {
    /// Associates a resolution failure with the committee epoch requested by the caller.
    fn new(target_epoch: EpochId, kind: CommitteeResolutionErrorKind) -> Self {
        Self { target_epoch, kind }
    }
}

/// Kind of committee resolution failure.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommitteeResolutionErrorKind {
    /// Fetching a committee directly from the trusted node failed.
    #[error("failed to fetch committee for epoch {epoch} from the trusted node")]
    FetchCommittee {
        /// Epoch requested from the node.
        epoch: EpochId,
        /// Underlying source error.
        #[source]
        source: BoxError,
    },
    /// The requested epoch predates the trusted committee anchor.
    #[error("target epoch is before trusted anchor epoch {anchor_epoch}")]
    TargetBeforeAnchor {
        /// Earliest epoch authenticated by the resolver.
        anchor_epoch: EpochId,
    },
    /// Fetching the node's current epoch failed.
    #[error("failed to fetch the node's current epoch")]
    FetchCurrentEpoch {
        /// Underlying source error.
        #[source]
        source: BoxError,
    },
    /// The service information response omitted the current epoch.
    #[error("service information is missing the current epoch")]
    MissingCurrentEpoch,
    /// The requested epoch is newer than the connected node's current epoch.
    #[error("target epoch is ahead of node current epoch {current_epoch}")]
    TargetAheadOfNode {
        /// Current epoch reported by the connected node.
        current_epoch: EpochId,
    },
    /// Fetching the certified summary that closed an epoch failed.
    #[error("failed to fetch end-of-epoch checkpoint information for epoch {epoch}")]
    FetchEpochHistory {
        /// Epoch whose certified closing summary was requested.
        epoch: EpochId,
        /// Underlying source error.
        #[source]
        source: BoxError,
    },
    /// A closed epoch response omitted its epoch-close proof.
    #[error("epoch {epoch} is missing its epoch-close proof")]
    MissingEpochCloseProof {
        /// Closed epoch whose proof was requested.
        epoch: EpochId,
    },
    /// The current trusted committee did not authenticate the end-of-epoch checkpoint.
    #[error("failed to verify epoch {epoch} end-of-epoch checkpoint {sequence_number}")]
    InvalidEndOfEpochCheckpoint {
        /// Epoch whose committee was used for verification.
        epoch: EpochId,
        /// Checkpoint sequence number closing the epoch.
        sequence_number: u64,
        /// Underlying checkpoint verification error.
        #[source]
        source: BoxError,
    },
    /// The epoch's last checkpoint did not contain next-epoch data.
    #[error("checkpoint {sequence_number} is not an end-of-epoch checkpoint")]
    NotEndOfEpoch {
        /// Checkpoint sequence number returned by the epoch response.
        sequence_number: u64,
    },
    /// Incrementing the authenticated epoch would overflow an [`EpochId`].
    #[error("next epoch after {epoch} overflows u64")]
    NextEpochOverflow {
        /// Authenticated checkpoint epoch.
        epoch: EpochId,
    },
    /// Reading or writing an authenticated committee in a cache failed.
    #[error("committee cache failed at epoch {epoch}")]
    Cache {
        /// Epoch being resolved through the cache.
        epoch: EpochId,
        /// Underlying cache error.
        #[source]
        source: CommitteeCacheError,
    },
}

/// Error returned when committee resolution or proof verification fails.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProofVerificationError {
    /// The committee required by the proof could not be resolved.
    #[error("failed to resolve the committee required by the proof")]
    CommitteeResolution {
        /// Committee-resolution failure.
        #[source]
        source: CommitteeResolutionError,
    },
    /// Offline proof verification failed.
    #[error("proof verification failed")]
    Proof {
        /// Offline verification failure.
        #[source]
        source: VerifyError,
    },
}

/// Selects how a resolver establishes trust in committee data.
#[derive(Clone)]
enum CommitteeResolution {
    /// Accept committee data returned directly by the connected node.
    Node,
    /// Authenticate committee lineage from an existing trust anchor.
    Anchor {
        committee: Committee,
        cache: Arc<dyn CommitteeCache>,
    },
}

/// Resolves the committee required to verify a checkpoint from a ledger source.
///
/// A resolver either accepts committee data directly from a trusted node or
/// starts from a trusted committee, normally obtained from the network genesis
/// blob, and authenticates every end-of-epoch handoff up to the requested epoch.
#[derive(Clone)]
pub struct CommitteeResolver<S> {
    source: S,
    mode: CommitteeResolution,
}

impl<S> CommitteeResolver<S>
where
    S: Source,
{
    /// Creates a resolver that trusts the connected node for committee data.
    ///
    /// This mode does not authenticate committee lineage. Use it only when the
    /// node is inside the caller's trust boundary, such as local development or
    /// explicitly trusted infrastructure.
    pub fn node(source: S) -> Self {
        Self {
            source,
            mode: CommitteeResolution::Node,
        }
    }

    /// Creates a resolver anchored at an already trusted committee.
    ///
    /// The trusted committee should be obtained from the network genesis blob
    /// or from a previously authenticated checkpoint. The connected node is
    /// treated only as a source of epoch and checkpoint data. Authenticated
    /// committees are retained in memory for subsequent resolutions.
    pub fn anchor(source: S, committee: Committee) -> Self {
        Self::anchor_with_cache(source, committee, MemoryCommitteeCache::new())
    }

    /// Creates an anchored resolver backed by a caller-provided committee cache.
    ///
    /// The cache is part of the caller's trust boundary and must return only
    /// committees authenticated for the same network. Committees fetched by
    /// this resolver are cached only after successful authentication.
    pub fn anchor_with_cache(source: S, committee: Committee, cache: impl CommitteeCache + 'static) -> Self {
        Self {
            source,
            mode: CommitteeResolution::Anchor {
                committee,
                cache: Arc::new(cache),
            },
        }
    }

    /// Resolves the authenticated committee for `target_epoch`.
    ///
    /// Node mode returns the committee reported by the trusted node. Anchor
    /// mode verifies each end-of-epoch checkpoint with the current committee
    /// before accepting its successor.
    pub async fn resolve(&self, target_epoch: EpochId) -> Result<Committee, CommitteeResolutionError> {
        match &self.mode {
            CommitteeResolution::Node => self.resolve_from_node(target_epoch).await,
            CommitteeResolution::Anchor { committee, cache } => {
                self.resolve_from_anchor(committee, cache.as_ref(), target_epoch).await
            }
        }
    }

    /// Resolves the committee required by `proof` and verifies the proof with it.
    ///
    /// Committee resolution may fetch committee or epoch-close evidence from
    /// the source. The final proof verification is performed locally by
    /// [`ProofVerifier`].
    pub async fn verify(&self, proof: &Proof) -> Result<(), ProofVerificationError> {
        let committee = self
            .resolve(proof.checkpoint_summary.epoch())
            .await
            .map_err(|source| ProofVerificationError::CommitteeResolution { source })?;

        ProofVerifier::new(&committee)
            .verify(proof)
            .map_err(|source| ProofVerificationError::Proof { source })
    }

    /// Fetches a committee directly from a node inside the caller's trust boundary.
    async fn resolve_from_node(&self, target_epoch: EpochId) -> Result<Committee, CommitteeResolutionError> {
        self.source.committee(target_epoch).await.map_err(|source| {
            CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::FetchCommittee {
                    epoch: target_epoch,
                    source: Box::new(source),
                },
            )
        })
    }

    /// Resolves from trusted cached committees before walking authenticated epoch summaries.
    async fn resolve_from_anchor(
        &self,
        trusted_committee: &Committee,
        cache: &dyn CommitteeCache,
        target_epoch: EpochId,
    ) -> Result<Committee, CommitteeResolutionError> {
        if target_epoch < trusted_committee.epoch {
            return Err(CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::TargetBeforeAnchor {
                    anchor_epoch: trusted_committee.epoch,
                },
            ));
        }

        if target_epoch == trusted_committee.epoch {
            return Ok(trusted_committee.clone());
        }

        if let Some(committee) = cache.committee(target_epoch).await.map_err(|source| {
            CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::Cache {
                    epoch: target_epoch,
                    source,
                },
            )
        })? {
            if committee.epoch != target_epoch {
                return Err(CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::Cache {
                        epoch: target_epoch,
                        source: CommitteeCacheError::Conflict { epoch: target_epoch },
                    },
                ));
            }

            return Ok(committee);
        }

        let mut committee = trusted_committee.clone();

        while committee.epoch < target_epoch {
            let next_epoch = committee.epoch + 1;
            let Some(cached) = cache.committee(next_epoch).await.map_err(|source| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::Cache {
                        epoch: next_epoch,
                        source,
                    },
                )
            })?
            else {
                break;
            };

            if cached.epoch != next_epoch {
                return Err(CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::Cache {
                        epoch: next_epoch,
                        source: CommitteeCacheError::Conflict { epoch: next_epoch },
                    },
                ));
            }

            committee = cached;
        }

        if committee.epoch == target_epoch {
            return Ok(committee);
        }

        let current_epoch = self.current_epoch(target_epoch).await?;
        if target_epoch > current_epoch {
            return Err(CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::TargetAheadOfNode { current_epoch },
            ));
        }

        while committee.epoch < target_epoch {
            let next_committee = self.fetch_next_committee(target_epoch, &committee, cache).await?;
            committee = next_committee;
        }

        Ok(committee)
    }

    /// Fetches the connected node's current epoch to reject unreachable targets early.
    async fn current_epoch(&self, target_epoch: EpochId) -> Result<EpochId, CommitteeResolutionError> {
        self.source
            .current_epoch()
            .await
            .map_err(|source| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::FetchCurrentEpoch {
                        source: Box::new(source),
                    },
                )
            })?
            .ok_or_else(|| {
                CommitteeResolutionError::new(target_epoch, CommitteeResolutionErrorKind::MissingCurrentEpoch)
            })
    }

    /// Fetches and authenticates the committee elected for the next epoch.
    async fn fetch_next_committee(
        &self,
        target_epoch: EpochId,
        current_committee: &Committee,
        cache: &dyn CommitteeCache,
    ) -> Result<Committee, CommitteeResolutionError> {
        let summary = self
            .source
            .epoch_close_summary(current_committee.epoch)
            .await
            .map_err(|source| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::FetchEpochHistory {
                        epoch: current_committee.epoch,
                        source: Box::new(source),
                    },
                )
            })?
            .ok_or_else(|| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::MissingEpochCloseProof {
                        epoch: current_committee.epoch,
                    },
                )
            })?;

        Self::authenticate_and_store_next_committee(target_epoch, current_committee, summary, cache).await
    }

    /// Verifies an end-of-epoch summary before accepting its next committee.
    fn authenticate_next_committee(
        current_committee: &Committee,
        summary: CertifiedCheckpointSummary,
    ) -> Result<Committee, CommitteeResolutionErrorKind> {
        let sequence_number = summary.sequence_number;
        let summary_epoch = summary.epoch();
        if summary_epoch != current_committee.epoch {
            return Err(CommitteeResolutionErrorKind::InvalidEndOfEpochCheckpoint {
                epoch: current_committee.epoch,
                sequence_number,
                source: Box::new(IotaError::WrongEpoch {
                    expected_epoch: current_committee.epoch,
                    actual_epoch: summary_epoch,
                }),
            });
        }

        if summary.end_of_epoch_data.is_none() {
            return Err(CommitteeResolutionErrorKind::NotEndOfEpoch { sequence_number });
        }

        let next_epoch = summary_epoch
            .checked_add(1)
            .ok_or(CommitteeResolutionErrorKind::NextEpochOverflow { epoch: summary_epoch })?;

        let verified = summary.try_into_verified(current_committee).map_err(|source| {
            CommitteeResolutionErrorKind::InvalidEndOfEpochCheckpoint {
                epoch: current_committee.epoch,
                sequence_number,
                source: Box::new(source),
            }
        })?;
        let next_epoch_committee = &verified
            .end_of_epoch_data
            .as_ref()
            .expect("checked before signature verification")
            .next_epoch_committee;

        Ok(Committee::from_committee_members(next_epoch, next_epoch_committee))
    }

    /// Authenticates a committee handoff before exposing it through the cache.
    async fn authenticate_and_store_next_committee(
        target_epoch: EpochId,
        current_committee: &Committee,
        summary: CertifiedCheckpointSummary,
        cache: &dyn CommitteeCache,
    ) -> Result<Committee, CommitteeResolutionError> {
        let next_committee = Self::authenticate_next_committee(current_committee, summary)
            .map_err(|kind| CommitteeResolutionError::new(target_epoch, kind))?;

        cache.store(&next_committee).await.map_err(|source| {
            CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::Cache {
                    epoch: next_committee.epoch,
                    source,
                },
            )
        })?;

        Ok(next_committee)
    }
}

#[cfg(feature = "native-grpc")]
impl CommitteeResolver<GrpcClient> {
    /// Returns the underlying SDK gRPC client.
    pub const fn grpc_client(&self) -> &GrpcClient {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use iota_sdk_types::{CheckpointSummary, EndOfEpochData, gas::GasCostSummary};

    use super::*;

    struct StaticCache {
        committee: Committee,
    }

    #[derive(Clone, Default)]
    struct RecordingCache {
        stored: Arc<Mutex<Vec<Committee>>>,
    }

    impl RecordingCache {
        fn stored(&self) -> Vec<Committee> {
            self.stored.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl CommitteeCache for RecordingCache {
        async fn committee(&self, _epoch: EpochId) -> Result<Option<Committee>, CommitteeCacheError> {
            Ok(None)
        }

        async fn store(&self, committee: &Committee) -> Result<(), CommitteeCacheError> {
            self.stored.lock().unwrap().push(committee.clone());
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl CommitteeCache for StaticCache {
        async fn committee(&self, epoch: EpochId) -> Result<Option<Committee>, CommitteeCacheError> {
            Ok((self.committee.epoch == epoch).then(|| self.committee.clone()))
        }

        async fn store(&self, _committee: &Committee) -> Result<(), CommitteeCacheError> {
            Ok(())
        }
    }

    fn signed_end_of_epoch_summary(
        current_epoch: EpochId,
        include_next_committee: bool,
    ) -> (Committee, Committee, CertifiedCheckpointSummary) {
        let (base_committee, keypairs) = Committee::new_simple_test_committee();
        let current_committee = Committee::new(current_epoch, base_committee.voting_rights.iter().cloned().collect());
        let (next_base_committee, _) = Committee::new_simple_test_committee_of_size(5);
        let next_committee = Committee::new(
            current_epoch.saturating_add(1),
            next_base_committee.voting_rights.iter().cloned().collect(),
        );
        let end_of_epoch_data = include_next_committee.then(|| EndOfEpochData {
            next_epoch_committee: next_committee.committee_members(),
            next_epoch_protocol_version: 1,
            epoch_commitments: Vec::new(),
            epoch_supply_change: 0,
        });
        let summary = CheckpointSummary {
            epoch: current_epoch,
            sequence_number: 42,
            network_total_transactions: 0,
            content_digest: Default::default(),
            previous_digest: None,
            epoch_rolling_gas_cost_summary: GasCostSummary::default(),
            timestamp_ms: 0,
            checkpoint_commitments: Vec::new(),
            end_of_epoch_data,
            version_specific_data: Vec::new(),
        };
        let certified_summary =
            CertifiedCheckpointSummary::new_from_keypairs_for_testing(summary, &keypairs, &current_committee);

        (current_committee, next_committee, certified_summary)
    }

    #[tokio::test]
    async fn authenticated_summary_stores_exactly_the_verified_committee() {
        let (current_committee, expected_committee, summary) = signed_end_of_epoch_summary(3, true);
        let cache = RecordingCache::default();

        let committee = CommitteeResolver::<GrpcClient>::authenticate_and_store_next_committee(
            4,
            &current_committee,
            summary,
            &cache,
        )
        .await
        .unwrap();

        assert_eq!(committee, expected_committee);
        assert_eq!(cache.stored(), vec![expected_committee]);
    }

    #[tokio::test]
    async fn invalid_checkpoint_signature_never_reaches_the_cache() {
        let (_, _, summary) = signed_end_of_epoch_summary(3, true);
        let (wrong_committee, _) = Committee::new_simple_test_committee_of_size(6);
        let wrong_committee = Committee::new(3, wrong_committee.voting_rights.iter().cloned().collect());
        let cache = RecordingCache::default();

        let error = CommitteeResolver::<GrpcClient>::authenticate_and_store_next_committee(
            4,
            &wrong_committee,
            summary,
            &cache,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::InvalidEndOfEpochCheckpoint {
                epoch: 3,
                sequence_number: 42,
                ..
            }
        ));
        assert!(cache.stored().is_empty());
    }

    #[tokio::test]
    async fn checkpoint_without_end_of_epoch_data_never_reaches_the_cache() {
        let (current_committee, _, summary) = signed_end_of_epoch_summary(3, false);
        let cache = RecordingCache::default();

        let error = CommitteeResolver::<GrpcClient>::authenticate_and_store_next_committee(
            4,
            &current_committee,
            summary,
            &cache,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::NotEndOfEpoch { sequence_number: 42 }
        ));
        assert!(cache.stored().is_empty());
    }

    #[tokio::test]
    async fn end_of_epoch_structure_is_checked_before_signatures() {
        let (_, _, summary) = signed_end_of_epoch_summary(3, false);
        let (wrong_committee, _) = Committee::new_simple_test_committee_of_size(6);
        let wrong_committee = Committee::new(3, wrong_committee.voting_rights.iter().cloned().collect());
        let cache = RecordingCache::default();

        let error = CommitteeResolver::<GrpcClient>::authenticate_and_store_next_committee(
            4,
            &wrong_committee,
            summary,
            &cache,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::NotEndOfEpoch { sequence_number: 42 }
        ));
        assert!(cache.stored().is_empty());
    }

    #[tokio::test]
    async fn wrong_epoch_summary_does_not_advance_or_reach_the_cache() {
        let (signing_committee, _, summary) = signed_end_of_epoch_summary(4, true);
        let expected_committee = Committee::new(3, signing_committee.voting_rights.iter().cloned().collect());
        let cache = RecordingCache::default();

        let error = CommitteeResolver::<GrpcClient>::authenticate_and_store_next_committee(
            4,
            &expected_committee,
            summary,
            &cache,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::InvalidEndOfEpochCheckpoint {
                epoch: 3,
                sequence_number: 42,
                ..
            }
        ));
        assert!(cache.stored().is_empty());
    }

    #[tokio::test]
    async fn overflowing_next_epoch_never_reaches_the_cache() {
        let (current_committee, _, summary) = signed_end_of_epoch_summary(EpochId::MAX, true);
        let cache = RecordingCache::default();

        let error = CommitteeResolver::<GrpcClient>::authenticate_and_store_next_committee(
            EpochId::MAX,
            &current_committee,
            summary,
            &cache,
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::NextEpochOverflow { epoch: EpochId::MAX }
        ));
        assert!(cache.stored().is_empty());
    }

    #[tokio::test]
    async fn node_resolution_mode_carries_no_anchored_cache() {
        let resolver = CommitteeResolver::node(GrpcClient::new("http://127.0.0.1:1").unwrap());

        assert!(matches!(resolver.mode, CommitteeResolution::Node));
    }

    #[tokio::test]
    async fn anchored_resolution_resumes_from_an_authenticated_cache() {
        let (current_committee, next_committee, summary) = signed_end_of_epoch_summary(3, true);
        let authenticated_committee =
            CommitteeResolver::<GrpcClient>::authenticate_next_committee(&current_committee, summary).unwrap();
        let cache = crate::MemoryCommitteeCache::new();
        cache.store(&authenticated_committee).await.unwrap();
        let client = GrpcClient::new("http://127.0.0.1:1").unwrap();
        let resolver = CommitteeResolver::anchor_with_cache(client, current_committee, cache);

        let resolved = resolver.resolve(4).await.unwrap();

        assert_eq!(resolved, next_committee);
    }

    #[tokio::test]
    async fn anchor_mode_uses_a_committee_cache_by_default() {
        let (current_committee, next_committee, summary) = signed_end_of_epoch_summary(3, true);
        let authenticated_committee =
            CommitteeResolver::<GrpcClient>::authenticate_next_committee(&current_committee, summary).unwrap();
        let client = GrpcClient::new("http://127.0.0.1:1").unwrap();
        let resolver = CommitteeResolver::anchor(client, current_committee);
        let CommitteeResolution::Anchor { cache, .. } = &resolver.mode else {
            panic!("anchor resolver must have a committee cache");
        };
        cache.store(&authenticated_committee).await.unwrap();

        let resolved = resolver.resolve(4).await.unwrap();

        assert_eq!(resolved, next_committee);
    }

    #[tokio::test]
    async fn memory_cache_rejects_a_conflicting_committee() {
        let (_, next_committee, _) = signed_end_of_epoch_summary(3, true);
        let cache = crate::MemoryCommitteeCache::new();
        cache.store(&next_committee).await.unwrap();
        let (conflicting_committee, _) = Committee::new_simple_test_committee_of_size(6);
        let conflicting_committee = Committee::new(4, conflicting_committee.voting_rights.iter().cloned().collect());

        let error = cache.store(&conflicting_committee).await.unwrap_err();

        assert!(matches!(error, CommitteeCacheError::Conflict { epoch: 4 }));
    }

    #[tokio::test]
    async fn anchored_resolution_accepts_a_committee_from_a_trusted_cache() {
        let (current_committee, next_committee, _) = signed_end_of_epoch_summary(3, true);
        let cache = StaticCache {
            committee: next_committee.clone(),
        };
        let client = GrpcClient::new("http://127.0.0.1:1").unwrap();
        let resolver = CommitteeResolver::anchor_with_cache(client, current_committee, cache);

        let resolved = resolver.resolve(4).await.unwrap();

        assert_eq!(resolved, next_committee);
    }
}
