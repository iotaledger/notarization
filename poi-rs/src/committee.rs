// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::io::Read;
use std::sync::Arc;

#[cfg(feature = "native-grpc")]
use iota_grpc_client::Client as GrpcClient;
use iota_sdk_types::CheckpointContents;
use iota_types::committee::{Committee, EpochId};
use iota_types::digests::ChainIdentifier;
use iota_types::effects::{TransactionEffects, TransactionEvents};
use iota_types::error::IotaError;
use iota_types::iota_system_state::{IotaSystemStateTrait, get_iota_system_state};
use iota_types::messages_checkpoint::CertifiedCheckpointSummary;
use iota_types::object::Object;
use iota_types::transaction::Transaction;
use serde::Deserialize;

use crate::{
    BoxError, CommitteeCache, CommitteeCacheError, CommitteeCacheKey, MemoryCommitteeCache, Proof, ProofVerifier,
    Source, VerifiedProof, VerifyError,
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
    /// Loading the initial committee from the trusted genesis blob failed.
    #[error("failed to load the committee from the trusted genesis blob")]
    LoadGenesisCommittee {
        /// Genesis decoding or system-state extraction failure.
        #[source]
        source: BoxError,
    },
    /// The trusted genesis checkpoint is not from epoch zero.
    #[error("trusted genesis checkpoint has epoch {epoch}, expected epoch 0")]
    UnexpectedGenesisCheckpointEpoch {
        /// Epoch encoded in the genesis checkpoint.
        epoch: EpochId,
    },
    /// The committee extracted from the trusted genesis blob is not from epoch zero.
    #[error("trusted genesis committee has epoch {epoch}, expected epoch 0")]
    UnexpectedGenesisCommitteeEpoch {
        /// Epoch encoded in the genesis committee.
        epoch: EpochId,
    },
    /// The trusted genesis checkpoint or its contents failed verification.
    #[error("trusted genesis checkpoint failed verification")]
    InvalidGenesisCheckpoint {
        /// Checkpoint signature or contents verification failure.
        #[source]
        source: BoxError,
    },
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
#[non_exhaustive]
pub enum CommitteeResolution {
    /// Accept committee data returned directly by the connected node.
    ///
    /// This does not authenticate committee lineage. Use it only when the node
    /// is inside the caller's trust boundary.
    TrustedNode,
    /// Authenticate committee lineage from an existing trust anchor.
    Anchored {
        /// First committee trusted by the caller.
        committee: Committee,
        /// Trusted network identity used to namespace a shared cache, or `None`
        /// when the cache is private to this resolution.
        chain_identifier: Option<ChainIdentifier>,
        /// Cache containing authenticated successor committees.
        cache: Arc<dyn CommitteeCache>,
    },
}

impl CommitteeResolution {
    /// Anchors committee resolution at an already trusted committee.
    ///
    /// Authenticated committees are retained in a fresh in-memory cache.
    pub fn anchored(committee: Committee) -> Self {
        Self::Anchored {
            committee,
            chain_identifier: None,
            cache: Arc::new(MemoryCommitteeCache::new()),
        }
    }

    /// Anchors committee resolution using a caller-provided committee cache.
    ///
    /// `chain_identifier` must be the trusted genesis checkpoint digest for the
    /// network containing `committee`. It namespaces entries in the shared cache.
    pub fn anchored_with_cache(
        chain_identifier: ChainIdentifier,
        committee: Committee,
        cache: impl CommitteeCache + 'static,
    ) -> Self {
        Self::Anchored {
            committee,
            chain_identifier: Some(chain_identifier),
            cache: Arc::new(cache),
        }
    }

    /// Anchors committee resolution at the committee contained in a trusted genesis blob.
    ///
    /// The reader must contain the BCS-encoded `genesis.blob` for the proof's
    /// network. Authenticated committees are retained in a fresh in-memory cache.
    pub fn from_genesis(reader: impl Read) -> Result<Self, CommitteeResolutionError> {
        Self::from_genesis_with_cache(reader, MemoryCommitteeCache::new())
    }

    /// Anchors committee resolution from a trusted genesis blob using a caller-provided cache.
    ///
    /// The reader must contain the BCS-encoded `genesis.blob` for the proof's
    /// network. Cache entries are scoped automatically to the genesis checkpoint digest.
    pub fn from_genesis_with_cache(
        reader: impl Read,
        cache: impl CommitteeCache + 'static,
    ) -> Result<Self, CommitteeResolutionError> {
        Self::load_genesis(reader, cache).map_err(|kind| CommitteeResolutionError::new(0, kind))
    }

    fn load_genesis(
        reader: impl Read,
        cache: impl CommitteeCache + 'static,
    ) -> Result<Self, CommitteeResolutionErrorKind> {
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

        let genesis: GenesisBlob =
            bcs::from_reader(reader).map_err(|source| CommitteeResolutionErrorKind::LoadGenesisCommittee {
                source: Box::new(source),
            })?;
        let checkpoint_epoch = genesis.checkpoint.epoch();
        if checkpoint_epoch != 0 {
            return Err(CommitteeResolutionErrorKind::UnexpectedGenesisCheckpointEpoch {
                epoch: checkpoint_epoch,
            });
        }

        let objects = genesis.objects.as_slice();
        let system_state =
            get_iota_system_state(&objects).map_err(|source| CommitteeResolutionErrorKind::LoadGenesisCommittee {
                source: Box::new(source),
            })?;
        let committee = system_state.get_current_epoch_committee().committee().clone();
        if committee.epoch != 0 {
            return Err(CommitteeResolutionErrorKind::UnexpectedGenesisCommitteeEpoch { epoch: committee.epoch });
        }

        genesis
            .checkpoint
            .verify_with_contents(&committee, Some(&genesis.checkpoint_contents))
            .map_err(|source| CommitteeResolutionErrorKind::InvalidGenesisCheckpoint {
                source: Box::new(source),
            })?;
        let chain_identifier = ChainIdentifier::from(*genesis.checkpoint.digest());

        Ok(Self::anchored_with_cache(chain_identifier, committee, cache))
    }
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
    /// Creates a resolver backed by `source` using `resolution` to establish committee trust.
    pub const fn new(source: S, resolution: CommitteeResolution) -> Self {
        Self {
            source,
            mode: resolution,
        }
    }

    /// Resolves the authenticated committee for `target_epoch`.
    ///
    /// Node mode returns the committee reported by the trusted node. Anchor
    /// mode verifies each end-of-epoch checkpoint with the current committee
    /// before accepting its successor.
    pub async fn resolve(&self, target_epoch: EpochId) -> Result<Committee, CommitteeResolutionError> {
        match &self.mode {
            CommitteeResolution::TrustedNode => self.resolve_from_node(target_epoch).await,
            CommitteeResolution::Anchored {
                committee,
                chain_identifier,
                cache,
            } => {
                self.resolve_from_anchor(committee, *chain_identifier, cache.as_ref(), target_epoch)
                    .await
            }
        }
    }

    /// Resolves the committee required by `proof` and verifies the proof with it.
    ///
    /// Committee resolution may fetch committee or epoch-close evidence from
    /// the source. The final proof verification is performed locally by
    /// [`ProofVerifier`]. On success, the returned [`VerifiedProof`] borrows the
    /// authenticated claims from `proof`.
    pub async fn verify<'proof>(&self, proof: &'proof Proof) -> Result<VerifiedProof<'proof>, ProofVerificationError> {
        let committee = self
            .resolve(proof.checkpoint_summary().epoch())
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
        chain_identifier: Option<ChainIdentifier>,
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

        let target_key = Self::cache_key(chain_identifier, target_epoch);
        if let Some(committee) = cache.committee(target_key).await.map_err(|source| {
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
            let next_key = Self::cache_key(chain_identifier, next_epoch);
            let Some(cached) = cache.committee(next_key).await.map_err(|source| {
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
            let next_committee = self
                .fetch_next_committee(target_epoch, chain_identifier, &committee, cache)
                .await?;
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
        chain_identifier: Option<ChainIdentifier>,
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

        let sequence_number = summary.sequence_number;
        let summary_epoch = summary.epoch();
        if summary_epoch != current_committee.epoch {
            return Err(CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::InvalidEndOfEpochCheckpoint {
                    epoch: current_committee.epoch,
                    sequence_number,
                    source: Box::new(IotaError::WrongEpoch {
                        expected_epoch: current_committee.epoch,
                        actual_epoch: summary_epoch,
                    }),
                },
            ));
        }

        if summary.end_of_epoch_data.is_none() {
            return Err(CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::NotEndOfEpoch { sequence_number },
            ));
        }

        let next_epoch = summary_epoch.checked_add(1).ok_or_else(|| {
            CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::NextEpochOverflow { epoch: summary_epoch },
            )
        })?;

        let verified = summary.try_into_verified(current_committee).map_err(|source| {
            CommitteeResolutionError::new(
                target_epoch,
                CommitteeResolutionErrorKind::InvalidEndOfEpochCheckpoint {
                    epoch: current_committee.epoch,
                    sequence_number,
                    source: Box::new(source),
                },
            )
        })?;
        let next_epoch_committee = &verified
            .end_of_epoch_data
            .as_ref()
            .expect("checked before signature verification")
            .next_epoch_committee;
        let next_committee = Committee::from_committee_members(next_epoch, next_epoch_committee);

        let cache_key = Self::cache_key(chain_identifier, next_committee.epoch);
        cache.store(cache_key, &next_committee).await.map_err(|source| {
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

    fn cache_key(chain_identifier: Option<ChainIdentifier>, epoch: EpochId) -> CommitteeCacheKey {
        chain_identifier.map_or_else(
            || CommitteeCacheKey::isolated(epoch),
            |chain| CommitteeCacheKey::new(chain, epoch),
        )
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
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use iota_sdk_types::gas::GasCostSummary;
    use iota_sdk_types::{CheckpointDigest, CheckpointSummary, EndOfEpochData, ObjectId, TransactionDigest, Version};
    use iota_types::digests::ChainIdentifier;
    use iota_types::messages_checkpoint::CertifiedCheckpointSummary;
    use iota_types::object::Object;

    use super::*;
    use crate::{SourceCheckpoint, SourceError, SourceTransaction};

    struct StaticCache {
        key: CommitteeCacheKey,
        committee: Committee,
    }

    struct FailingStoreCache;

    #[derive(Clone)]
    struct EpochCloseSource {
        summary: CertifiedCheckpointSummary,
    }

    #[derive(Clone)]
    struct CommitteeHistorySource {
        current_epoch: Option<EpochId>,
        summaries: BTreeMap<EpochId, CertifiedCheckpointSummary>,
        fail_current_epoch: bool,
    }

    #[async_trait::async_trait]
    impl Source for EpochCloseSource {
        async fn chain_identifier(&self) -> Result<ChainIdentifier, SourceError> {
            unreachable!("committee transition does not resolve a chain identifier")
        }

        async fn transaction(
            &self,
            _transaction_digest: TransactionDigest,
        ) -> Result<Option<SourceTransaction>, SourceError> {
            unreachable!("committee transition does not resolve transactions")
        }

        async fn object(&self, _object_id: ObjectId, _version: Option<Version>) -> Result<Option<Object>, SourceError> {
            unreachable!("committee transition does not resolve objects")
        }

        async fn checkpoint(&self, _sequence_number: u64) -> Result<Option<SourceCheckpoint>, SourceError> {
            unreachable!("committee transition does not resolve checkpoints")
        }

        async fn committee(&self, _epoch: EpochId) -> Result<Committee, SourceError> {
            unreachable!("anchored committee transition does not trust node committees")
        }

        async fn current_epoch(&self) -> Result<Option<EpochId>, SourceError> {
            Ok(Some(self.summary.epoch().saturating_add(1)))
        }

        async fn epoch_close_summary(
            &self,
            _epoch: EpochId,
        ) -> Result<Option<CertifiedCheckpointSummary>, SourceError> {
            Ok(Some(self.summary.clone()))
        }
    }

    #[async_trait::async_trait]
    impl Source for CommitteeHistorySource {
        async fn chain_identifier(&self) -> Result<ChainIdentifier, SourceError> {
            unreachable!("committee history does not resolve a chain identifier")
        }

        async fn transaction(
            &self,
            _transaction_digest: TransactionDigest,
        ) -> Result<Option<SourceTransaction>, SourceError> {
            unreachable!("committee history does not resolve transactions")
        }

        async fn object(&self, _object_id: ObjectId, _version: Option<Version>) -> Result<Option<Object>, SourceError> {
            unreachable!("committee history does not resolve objects")
        }

        async fn checkpoint(&self, _sequence_number: u64) -> Result<Option<SourceCheckpoint>, SourceError> {
            unreachable!("committee history does not resolve checkpoints")
        }

        async fn committee(&self, _epoch: EpochId) -> Result<Committee, SourceError> {
            unreachable!("anchored committee history does not trust node committees")
        }

        async fn current_epoch(&self) -> Result<Option<EpochId>, SourceError> {
            if self.fail_current_epoch {
                return Err(SourceError::request(std::io::Error::other("current epoch unavailable")));
            }

            Ok(self.current_epoch)
        }

        async fn epoch_close_summary(&self, epoch: EpochId) -> Result<Option<CertifiedCheckpointSummary>, SourceError> {
            Ok(self.summaries.get(&epoch).cloned())
        }
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
        async fn committee(&self, _key: CommitteeCacheKey) -> Result<Option<Committee>, CommitteeCacheError> {
            Ok(None)
        }

        async fn store(&self, _key: CommitteeCacheKey, committee: &Committee) -> Result<(), CommitteeCacheError> {
            self.stored.lock().unwrap().push(committee.clone());
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl CommitteeCache for StaticCache {
        async fn committee(&self, key: CommitteeCacheKey) -> Result<Option<Committee>, CommitteeCacheError> {
            Ok((self.key == key).then(|| self.committee.clone()))
        }

        async fn store(&self, _key: CommitteeCacheKey, _committee: &Committee) -> Result<(), CommitteeCacheError> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl CommitteeCache for FailingStoreCache {
        async fn committee(&self, _key: CommitteeCacheKey) -> Result<Option<Committee>, CommitteeCacheError> {
            Ok(None)
        }

        async fn store(&self, key: CommitteeCacheKey, _committee: &Committee) -> Result<(), CommitteeCacheError> {
            Err(CommitteeCacheError::Backend {
                epoch: key.epoch(),
                source: Box::new(std::io::Error::other("cache unavailable")),
            })
        }
    }

    fn chain_identifier(byte: u8) -> ChainIdentifier {
        ChainIdentifier::from(CheckpointDigest::new([byte; 32]))
    }

    fn committee_with_keypairs(epoch: EpochId, size: usize) -> (Committee, Vec<iota_types::crypto::AuthorityKeyPair>) {
        let (base_committee, keypairs) = Committee::new_simple_test_committee_of_size(size);
        let committee = Committee::new(epoch, base_committee.voting_rights.iter().cloned().collect());

        (committee, keypairs)
    }

    fn signed_committee_transition(
        current: &Committee,
        keypairs: &[iota_types::crypto::AuthorityKeyPair],
        next: &Committee,
    ) -> CertifiedCheckpointSummary {
        let summary = CheckpointSummary {
            epoch: current.epoch,
            sequence_number: current.epoch,
            network_total_transactions: 0,
            contents_digest: Default::default(),
            previous_digest: None,
            epoch_rolling_gas_cost_summary: GasCostSummary::default(),
            timestamp_ms: 0,
            checkpoint_commitments: Vec::new(),
            end_of_epoch_data: Some(EndOfEpochData {
                next_epoch_committee: next.committee_members(),
                next_epoch_protocol_version: 1,
                epoch_commitments: Vec::new(),
                epoch_supply_change: 0,
            }),
            version_specific_data: Vec::new(),
        };

        CertifiedCheckpointSummary::new_from_keypairs_for_testing(summary, keypairs, current)
    }

    fn signed_end_of_epoch_summary(
        current_epoch: EpochId,
        include_next_committee: bool,
    ) -> (Committee, Committee, CertifiedCheckpointSummary) {
        let (base_committee, keypairs) = Committee::new_simple_test_committee();
        signed_end_of_epoch_summary_from_test_committee(
            current_epoch,
            include_next_committee,
            base_committee,
            keypairs,
            5,
        )
    }

    fn signed_end_of_epoch_summary_with_sizes(
        current_epoch: EpochId,
        include_next_committee: bool,
        current_committee_size: usize,
        next_committee_size: usize,
    ) -> (Committee, Committee, CertifiedCheckpointSummary) {
        let (base_committee, keypairs) = Committee::new_simple_test_committee_of_size(current_committee_size);
        signed_end_of_epoch_summary_from_test_committee(
            current_epoch,
            include_next_committee,
            base_committee,
            keypairs,
            next_committee_size,
        )
    }

    fn signed_end_of_epoch_summary_from_test_committee(
        current_epoch: EpochId,
        include_next_committee: bool,
        base_committee: Committee,
        keypairs: Vec<iota_types::crypto::AuthorityKeyPair>,
        next_committee_size: usize,
    ) -> (Committee, Committee, CertifiedCheckpointSummary) {
        let current_committee = Committee::new(current_epoch, base_committee.voting_rights.iter().cloned().collect());
        let (next_base_committee, _) = Committee::new_simple_test_committee_of_size(next_committee_size);
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
            contents_digest: Default::default(),
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
        let resolver = CommitteeResolver::new(
            EpochCloseSource { summary },
            CommitteeResolution::anchored(current_committee.clone()),
        );

        let committee = resolver
            .fetch_next_committee(4, None, &current_committee, &cache)
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
        let resolver = CommitteeResolver::new(
            EpochCloseSource { summary },
            CommitteeResolution::anchored(wrong_committee.clone()),
        );

        let error = resolver
            .fetch_next_committee(4, None, &wrong_committee, &cache)
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
        let resolver = CommitteeResolver::new(
            EpochCloseSource { summary },
            CommitteeResolution::anchored(current_committee.clone()),
        );

        let error = resolver
            .fetch_next_committee(4, None, &current_committee, &cache)
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
        let resolver = CommitteeResolver::new(
            EpochCloseSource { summary },
            CommitteeResolution::anchored(wrong_committee.clone()),
        );

        let error = resolver
            .fetch_next_committee(4, None, &wrong_committee, &cache)
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
        let resolver = CommitteeResolver::new(
            EpochCloseSource { summary },
            CommitteeResolution::anchored(expected_committee.clone()),
        );

        let error = resolver
            .fetch_next_committee(4, None, &expected_committee, &cache)
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
        let resolver = CommitteeResolver::new(
            EpochCloseSource { summary },
            CommitteeResolution::anchored(current_committee.clone()),
        );

        let error = resolver
            .fetch_next_committee(EpochId::MAX, None, &current_committee, &cache)
            .await
            .unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::NextEpochOverflow { epoch: EpochId::MAX }
        ));
        assert!(cache.stored().is_empty());
    }

    #[tokio::test]
    async fn anchor_mode_uses_a_committee_cache_by_default() {
        let (current_committee, next_committee, _) = signed_end_of_epoch_summary(3, true);
        let client = GrpcClient::new("http://127.0.0.1:1").unwrap();
        let resolver = CommitteeResolver::new(client, CommitteeResolution::anchored(current_committee));
        let CommitteeResolution::Anchored { cache, .. } = &resolver.mode else {
            panic!("anchor resolver must have a committee cache");
        };
        cache
            .store(CommitteeCacheKey::isolated(next_committee.epoch), &next_committee)
            .await
            .unwrap();

        let resolved = resolver.resolve(4).await.unwrap();

        assert_eq!(resolved, next_committee);
    }

    #[tokio::test]
    async fn anchored_resolution_accepts_a_committee_from_a_trusted_cache() {
        let (current_committee, next_committee, _) = signed_end_of_epoch_summary(3, true);
        let chain_identifier = chain_identifier(1);
        let cache = StaticCache {
            key: CommitteeCacheKey::new(chain_identifier, next_committee.epoch),
            committee: next_committee.clone(),
        };
        let client = GrpcClient::new("http://127.0.0.1:1").unwrap();
        let resolver = CommitteeResolver::new(
            client,
            CommitteeResolution::anchored_with_cache(chain_identifier, current_committee, cache),
        );

        let resolved = resolver.resolve(4).await.unwrap();

        assert_eq!(resolved, next_committee);
    }

    #[tokio::test]
    async fn target_cache_entry_must_contain_the_requested_epoch() {
        let (anchor, _) = committee_with_keypairs(3, 4);
        let (mislabeled, _) = committee_with_keypairs(5, 5);
        let chain_identifier = chain_identifier(1);
        let cache = StaticCache {
            key: CommitteeCacheKey::new(chain_identifier, 4),
            committee: mislabeled,
        };
        let resolver = CommitteeResolver::new(
            CommitteeHistorySource {
                current_epoch: Some(5),
                summaries: BTreeMap::new(),
                fail_current_epoch: false,
            },
            CommitteeResolution::anchored_with_cache(chain_identifier, anchor, cache),
        );

        let error = resolver.resolve(4).await.unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::Cache {
                epoch: 4,
                source: CommitteeCacheError::Conflict { epoch: 4 }
            }
        ));
    }

    #[tokio::test]
    async fn intermediate_cache_entry_must_contain_its_key_epoch() {
        let (anchor, _) = committee_with_keypairs(3, 4);
        let (mislabeled, _) = committee_with_keypairs(5, 5);
        let chain_identifier = chain_identifier(1);
        let cache = StaticCache {
            key: CommitteeCacheKey::new(chain_identifier, 4),
            committee: mislabeled,
        };
        let resolver = CommitteeResolver::new(
            CommitteeHistorySource {
                current_epoch: Some(5),
                summaries: BTreeMap::new(),
                fail_current_epoch: false,
            },
            CommitteeResolution::anchored_with_cache(chain_identifier, anchor, cache),
        );

        let error = resolver.resolve(5).await.unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::Cache {
                epoch: 4,
                source: CommitteeCacheError::Conflict { epoch: 4 }
            }
        ));
    }

    #[tokio::test]
    async fn anchored_resolution_reports_a_current_epoch_source_failure() {
        let (anchor, _) = committee_with_keypairs(0, 4);
        let resolver = CommitteeResolver::new(
            CommitteeHistorySource {
                current_epoch: None,
                summaries: BTreeMap::new(),
                fail_current_epoch: true,
            },
            CommitteeResolution::anchored(anchor),
        );

        let error = resolver.resolve(1).await.unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::FetchCurrentEpoch { .. }
        ));
    }

    #[tokio::test]
    async fn anchored_resolution_reports_missing_epoch_close_evidence() {
        let (anchor, _) = committee_with_keypairs(0, 4);
        let resolver = CommitteeResolver::new(
            CommitteeHistorySource {
                current_epoch: Some(1),
                summaries: BTreeMap::new(),
                fail_current_epoch: false,
            },
            CommitteeResolution::anchored(anchor),
        );

        let error = resolver.resolve(1).await.unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::MissingEpochCloseProof { epoch: 0 }
        ));
    }

    #[tokio::test]
    async fn anchored_resolution_reports_a_cache_store_failure() {
        let (anchor, keypairs) = committee_with_keypairs(0, 4);
        let (next, _) = committee_with_keypairs(1, 5);
        let summary = signed_committee_transition(&anchor, &keypairs, &next);
        let resolver = CommitteeResolver::new(
            CommitteeHistorySource {
                current_epoch: Some(1),
                summaries: BTreeMap::from([(0, summary)]),
                fail_current_epoch: false,
            },
            CommitteeResolution::anchored_with_cache(chain_identifier(1), anchor, FailingStoreCache),
        );

        let error = resolver.resolve(1).await.unwrap_err();

        assert!(matches!(
            error.kind,
            CommitteeResolutionErrorKind::Cache {
                epoch: 1,
                source: CommitteeCacheError::Backend { epoch: 1, .. }
            }
        ));
    }

    #[tokio::test]
    async fn anchored_resolution_authenticates_multiple_epoch_transitions() {
        let (first, first_keypairs) = committee_with_keypairs(0, 4);
        let (second, second_keypairs) = committee_with_keypairs(1, 5);
        let (expected, _) = committee_with_keypairs(2, 6);
        let summaries = BTreeMap::from([
            (0, signed_committee_transition(&first, &first_keypairs, &second)),
            (1, signed_committee_transition(&second, &second_keypairs, &expected)),
        ]);
        let resolver = CommitteeResolver::new(
            CommitteeHistorySource {
                current_epoch: Some(2),
                summaries,
                fail_current_epoch: false,
            },
            CommitteeResolution::anchored(first),
        );

        let resolved = resolver.resolve(2).await.unwrap();

        assert_eq!(resolved, expected);
    }

    #[tokio::test]
    async fn shared_cache_isolated_between_distinct_networks() {
        let (_, first_successor, _) = signed_end_of_epoch_summary(3, true);
        let (second_anchor, second_successor, second_summary) = signed_end_of_epoch_summary_with_sizes(3, true, 6, 6);
        let first_chain = chain_identifier(1);
        let second_chain = chain_identifier(2);
        let cache = MemoryCommitteeCache::new();
        cache
            .store(
                CommitteeCacheKey::new(first_chain, first_successor.epoch),
                &first_successor,
            )
            .await
            .unwrap();
        let resolver = CommitteeResolver::new(
            EpochCloseSource {
                summary: second_summary,
            },
            CommitteeResolution::anchored_with_cache(second_chain, second_anchor.clone(), cache.clone()),
        );

        let resolved = resolver.resolve(4).await.unwrap();

        assert_eq!(resolved, second_successor);
        assert_ne!(resolved, first_successor);
        assert_eq!(cache.len().await, 2);
        assert_eq!(
            cache
                .committee(CommitteeCacheKey::new(second_chain, resolved.epoch))
                .await
                .unwrap(),
            Some(resolved)
        );
    }
}
