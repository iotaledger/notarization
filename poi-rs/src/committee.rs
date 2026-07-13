// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use iota_grpc_client::{
    Client as GrpcClient, ReadMask,
    read_mask_fields::{CheckpointResponseField, EpochField, ServiceInfoField},
};
use iota_types::{
    committee::{Committee, EpochId},
    messages_checkpoint::{CertifiedCheckpointSummary, EndOfEpochData},
};

use crate::{BoxError, CommitteeCache, CommitteeCacheError, MemoryCommitteeCache};

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
        /// Underlying gRPC error.
        #[source]
        source: BoxError,
    },
    /// Reading a committee returned by the trusted node failed.
    #[error("failed to read committee for epoch {epoch} from the trusted node")]
    Committee {
        /// Epoch requested from the node.
        epoch: EpochId,
        /// Underlying response error.
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
        /// Underlying gRPC error.
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
    /// Fetching the last checkpoint of an epoch failed.
    #[error("failed to fetch end-of-epoch checkpoint information for epoch {epoch}")]
    FetchEpochHistory {
        /// Epoch whose last checkpoint was requested.
        epoch: EpochId,
        /// Underlying gRPC error.
        #[source]
        source: BoxError,
    },
    /// The epoch response omitted its last checkpoint sequence number.
    #[error("epoch {epoch} is missing its last checkpoint")]
    MissingLastCheckpoint {
        /// Epoch whose last checkpoint was requested.
        epoch: EpochId,
    },
    /// Fetching a certified end-of-epoch checkpoint summary failed.
    #[error("failed to fetch end-of-epoch checkpoint {sequence_number}")]
    FetchCheckpoint {
        /// Checkpoint sequence number requested from the node.
        sequence_number: u64,
        /// Underlying gRPC error.
        #[source]
        source: BoxError,
    },
    /// Reading or converting a checkpoint summary failed.
    #[error("failed to read end-of-epoch checkpoint {sequence_number}")]
    CheckpointSummary {
        /// Checkpoint sequence number returned by the epoch response.
        sequence_number: u64,
        /// Underlying response or conversion error.
        #[source]
        source: BoxError,
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

/// Selects how a resolver establishes trust in committee data.
#[derive(Clone)]
enum TrustMode {
    /// Accept committee data returned directly by the connected node.
    Node,
    /// Authenticate committee lineage from an existing trust anchor.
    Anchor {
        committee: Committee,
        cache: Arc<dyn CommitteeCache>,
    },
}

/// Resolves the committee required to verify a checkpoint from a gRPC node.
///
/// A resolver either accepts committee data directly from a trusted node or
/// starts from a trusted committee, normally obtained from the network genesis
/// blob, and authenticates every end-of-epoch handoff up to the requested epoch.
#[derive(Clone)]
pub struct CommitteeResolver {
    client: GrpcClient,
    mode: TrustMode,
}

impl CommitteeResolver {
    /// Creates a resolver that trusts the connected node for committee data.
    ///
    /// This mode does not authenticate committee lineage. Use it only when the
    /// node is inside the caller's trust boundary, such as local development or
    /// explicitly trusted infrastructure.
    pub fn node(client: GrpcClient) -> Self {
        Self {
            client,
            mode: TrustMode::Node,
        }
    }

    /// Creates a resolver anchored at an already trusted committee.
    ///
    /// The trusted committee should be obtained from the network genesis blob
    /// or from a previously authenticated checkpoint. The connected node is
    /// treated only as a source of epoch and checkpoint data. Authenticated
    /// committees are retained in memory for subsequent resolutions.
    pub fn anchor(client: GrpcClient, committee: Committee) -> Self {
        Self::anchor_with_cache(client, committee, MemoryCommitteeCache::new())
    }

    /// Creates an anchored resolver backed by a caller-provided committee cache.
    ///
    /// The cache is part of the caller's trust boundary and must return only
    /// committees authenticated for the same network. Committees fetched by
    /// this resolver are cached only after successful authentication.
    pub fn anchor_with_cache(client: GrpcClient, committee: Committee, cache: impl CommitteeCache + 'static) -> Self {
        Self {
            client,
            mode: TrustMode::Anchor {
                committee,
                cache: Arc::new(cache),
            },
        }
    }

    /// Returns the underlying SDK gRPC client.
    pub const fn grpc_client(&self) -> &GrpcClient {
        &self.client
    }

    /// Resolves the authenticated committee for `target_epoch`.
    ///
    /// Node mode returns the committee reported by the trusted node. Anchor
    /// mode verifies each end-of-epoch checkpoint with the current committee
    /// before accepting its successor.
    pub async fn resolve(&self, target_epoch: EpochId) -> Result<Committee, CommitteeResolutionError> {
        match &self.mode {
            TrustMode::Node => self.resolve_from_node(target_epoch).await,
            TrustMode::Anchor { committee, cache } => {
                self.resolve_from_anchor(committee, cache.as_ref(), target_epoch).await
            }
        }
    }

    /// Fetches a committee directly from a node inside the caller's trust boundary.
    async fn resolve_from_node(&self, target_epoch: EpochId) -> Result<Committee, CommitteeResolutionError> {
        let epoch = self
            .client
            .get_epoch(Some(target_epoch), Some(ReadMask::from(EpochField::COMMITTEE)))
            .await
            .map_err(|source| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::FetchCommittee {
                        epoch: target_epoch,
                        source: Box::new(source),
                    },
                )
            })?
            .into_inner();
        let committee = epoch.committee().map_err(|source| {
            CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::Committee {
                    epoch: target_epoch,
                    source: Box::new(source),
                },
            )
        })?;

        Ok(committee.into())
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
            let next_committee = self.fetch_next_committee(target_epoch, &committee).await?;
            cache.store(&next_committee).await.map_err(|source| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::Cache {
                        epoch: next_committee.epoch,
                        source,
                    },
                )
            })?;
            committee = next_committee;
        }

        Ok(committee)
    }

    /// Fetches the connected node's current epoch to reject unreachable targets early.
    async fn current_epoch(&self, target_epoch: EpochId) -> Result<EpochId, CommitteeResolutionError> {
        self.client
            .get_service_info(Some(ReadMask::from(ServiceInfoField::EPOCH)))
            .await
            .map_err(|source| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::FetchCurrentEpoch {
                        source: Box::new(source),
                    },
                )
            })?
            .body()
            .epoch
            .ok_or_else(|| {
                CommitteeResolutionError::new(target_epoch, CommitteeResolutionErrorKind::MissingCurrentEpoch)
            })
    }

    /// Fetches and authenticates the committee elected for the next epoch.
    async fn fetch_next_committee(
        &self,
        target_epoch: EpochId,
        current_committee: &Committee,
    ) -> Result<Committee, CommitteeResolutionError> {
        let sequence_number = self
            .epoch_last_checkpoint(target_epoch, current_committee.epoch)
            .await?;
        let summary = self.certified_checkpoint_summary(target_epoch, sequence_number).await?;
        let next_committee = Self::authenticate_next_committee(current_committee, &summary)
            .map_err(|kind| CommitteeResolutionError::new(target_epoch, kind))?;

        Ok(next_committee)
    }

    /// Fetches the checkpoint sequence number that closes an epoch.
    async fn epoch_last_checkpoint(
        &self,
        target_epoch: EpochId,
        epoch: EpochId,
    ) -> Result<u64, CommitteeResolutionError> {
        self.client
            .get_epoch(Some(epoch), Some(ReadMask::from(EpochField::LAST_CHECKPOINT)))
            .await
            .map_err(|source| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::FetchEpochHistory {
                        epoch,
                        source: Box::new(source),
                    },
                )
            })?
            .into_inner()
            .last_checkpoint
            .ok_or_else(|| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::MissingLastCheckpoint { epoch },
                )
            })
    }

    /// Fetches only the signed checkpoint summary required to authenticate the next committee.
    async fn certified_checkpoint_summary(
        &self,
        target_epoch: EpochId,
        sequence_number: u64,
    ) -> Result<CertifiedCheckpointSummary, CommitteeResolutionError> {
        let checkpoint = self
            .client
            .get_checkpoint_by_sequence_number(
                sequence_number,
                Some(ReadMask::from(CHECKPOINT_SUMMARY_FIELDS)),
                None,
                None,
            )
            .await
            .map_err(|source| {
                CommitteeResolutionError::new(
                    target_epoch,
                    CommitteeResolutionErrorKind::FetchCheckpoint {
                        sequence_number,
                        source: Box::new(source),
                    },
                )
            })?
            .into_inner();

        let summary = checkpoint.signed_summary().map_err(|source| {
            CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::CheckpointSummary {
                    sequence_number,
                    source: Box::new(source),
                },
            )
        })?;

        summary.try_into().map_err(|source| {
            CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::CheckpointSummary {
                    sequence_number,
                    source: Box::new(source),
                },
            )
        })
    }

    /// Verifies an end-of-epoch summary before accepting its next committee.
    fn authenticate_next_committee(
        current_committee: &Committee,
        summary: &CertifiedCheckpointSummary,
    ) -> Result<Committee, CommitteeResolutionErrorKind> {
        let sequence_number = summary.sequence_number;
        summary.clone().try_into_verified(current_committee).map_err(|source| {
            CommitteeResolutionErrorKind::InvalidEndOfEpochCheckpoint {
                epoch: current_committee.epoch,
                sequence_number,
                source: Box::new(source),
            }
        })?;

        let Some(EndOfEpochData {
            next_epoch_committee, ..
        }) = &summary.end_of_epoch_data
        else {
            return Err(CommitteeResolutionErrorKind::NotEndOfEpoch { sequence_number });
        };
        let next_epoch = summary
            .epoch()
            .checked_add(1)
            .ok_or(CommitteeResolutionErrorKind::NextEpochOverflow { epoch: summary.epoch() })?;

        Ok(Committee::new(
            next_epoch,
            next_epoch_committee.iter().cloned().collect(),
        ))
    }
}

/// Checkpoint fields required to authenticate the next committee.
const CHECKPOINT_SUMMARY_FIELDS: &[&str] = &[
    CheckpointResponseField::CHECKPOINT_SUMMARY_BCS,
    CheckpointResponseField::CHECKPOINT_SIGNATURE,
];

#[cfg(test)]
mod tests {
    use iota_sdk_types::gas::GasCostSummary;
    use iota_types::messages_checkpoint::{CheckpointSummary, EndOfEpochData};

    use super::*;

    struct StaticCache {
        committee: Committee,
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
            current_epoch + 1,
            next_base_committee.voting_rights.iter().cloned().collect(),
        );
        let end_of_epoch_data = include_next_committee.then(|| EndOfEpochData {
            next_epoch_committee: next_committee.voting_rights.clone(),
            next_epoch_protocol_version: 1.into(),
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

    #[test]
    fn authenticated_summary_returns_the_next_committee() {
        let (current_committee, expected_committee, summary) = signed_end_of_epoch_summary(3, true);

        let committee = CommitteeResolver::authenticate_next_committee(&current_committee, &summary).unwrap();

        assert_eq!(committee, expected_committee);
    }

    #[test]
    fn summary_rejects_a_signature_from_another_committee() {
        let (_, _, summary) = signed_end_of_epoch_summary(3, true);
        let (wrong_committee, _) = Committee::new_simple_test_committee_of_size(6);
        let wrong_committee = Committee::new(3, wrong_committee.voting_rights.iter().cloned().collect());

        let error = CommitteeResolver::authenticate_next_committee(&wrong_committee, &summary).unwrap_err();

        assert!(matches!(
            error,
            CommitteeResolutionErrorKind::InvalidEndOfEpochCheckpoint {
                epoch: 3,
                sequence_number: 42,
                ..
            }
        ));
    }

    #[test]
    fn summary_requires_end_of_epoch_data() {
        let (current_committee, _, summary) = signed_end_of_epoch_summary(3, false);

        let error = CommitteeResolver::authenticate_next_committee(&current_committee, &summary).unwrap_err();

        assert!(matches!(
            error,
            CommitteeResolutionErrorKind::NotEndOfEpoch { sequence_number: 42 }
        ));
    }

    #[tokio::test]
    async fn anchored_resolution_resumes_from_an_authenticated_cache() {
        let (current_committee, next_committee, summary) = signed_end_of_epoch_summary(3, true);
        let authenticated_committee =
            CommitteeResolver::authenticate_next_committee(&current_committee, &summary).unwrap();
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
            CommitteeResolver::authenticate_next_committee(&current_committee, &summary).unwrap();
        let client = GrpcClient::new("http://127.0.0.1:1").unwrap();
        let resolver = CommitteeResolver::anchor(client, current_committee);
        let TrustMode::Anchor { cache, .. } = &resolver.mode else {
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
