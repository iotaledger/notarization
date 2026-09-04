// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::sync::Arc;

use iota_types::committee::Committee;
#[cfg(test)]
use iota_types::committee::EpochId;
use tokio::sync::RwLock;

use super::{CommitteeCache, CommitteeCacheError, CommitteeCacheKey};

/// In-memory committee cache for application use and tests.
#[derive(Clone, Debug, Default)]
pub struct MemoryCommitteeCache {
    committees: Arc<RwLock<BTreeMap<CommitteeCacheKey, Committee>>>,
}

impl MemoryCommitteeCache {
    /// Creates an empty in-memory committee cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of cached committees.
    pub async fn len(&self) -> usize {
        self.committees.read().await.len()
    }

    /// Returns whether the cache contains no committees.
    pub async fn is_empty(&self) -> bool {
        self.committees.read().await.is_empty()
    }
}

#[async_trait::async_trait]
impl CommitteeCache for MemoryCommitteeCache {
    async fn committee(&self, key: CommitteeCacheKey) -> Result<Option<Committee>, CommitteeCacheError> {
        Ok(self.committees.read().await.get(&key).cloned())
    }

    async fn store(&self, key: CommitteeCacheKey, committee: &Committee) -> Result<(), CommitteeCacheError> {
        let epoch = committee.epoch;
        if key.epoch() != epoch {
            return Err(CommitteeCacheError::Conflict { epoch });
        }

        let mut committees = self.committees.write().await;

        if committees.get(&key).is_some_and(|cached| cached != committee) {
            return Err(CommitteeCacheError::Conflict { epoch });
        }

        committees.entry(key).or_insert_with(|| committee.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use iota_types::digests::ChainIdentifier;

    use super::*;

    fn committee_at(epoch: EpochId) -> Committee {
        let (committee, _) = Committee::new_simple_test_committee();

        Committee::new(epoch, committee.voting_rights.iter().cloned().collect())
    }

    fn chain_identifier(byte: u8) -> ChainIdentifier {
        ChainIdentifier::from(iota_sdk_types::CheckpointDigest::new([byte; 32]))
    }

    fn key(chain_identifier: ChainIdentifier, epoch: EpochId) -> CommitteeCacheKey {
        CommitteeCacheKey::new(chain_identifier, epoch)
    }

    #[tokio::test]
    async fn new_cache_is_empty() {
        let cache = MemoryCommitteeCache::new();
        let chain_identifier = chain_identifier(1);

        assert!(cache.is_empty().await);
        assert_eq!(cache.len().await, 0);
        assert!(cache.committee(key(chain_identifier, 7)).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn store_makes_a_committee_available_by_epoch() {
        let cache = MemoryCommitteeCache::new();
        let chain_identifier = chain_identifier(1);
        let committee = committee_at(7);

        cache.store(key(chain_identifier, 7), &committee).await.unwrap();

        assert_eq!(
            cache.committee(key(chain_identifier, 7)).await.unwrap(),
            Some(committee)
        );
        assert_eq!(cache.len().await, 1);
        assert!(!cache.is_empty().await);
    }

    #[tokio::test]
    async fn storing_the_same_committee_is_idempotent() {
        let cache = MemoryCommitteeCache::new();
        let chain_identifier = chain_identifier(1);
        let committee = committee_at(7);

        cache.store(key(chain_identifier, 7), &committee).await.unwrap();
        cache.store(key(chain_identifier, 7), &committee).await.unwrap();

        assert_eq!(
            cache.committee(key(chain_identifier, 7)).await.unwrap(),
            Some(committee)
        );
        assert_eq!(cache.len().await, 1);
    }

    #[tokio::test]
    async fn conflicting_committee_is_rejected_without_replacing_the_original() {
        let cache = MemoryCommitteeCache::new();
        let chain_identifier = chain_identifier(1);
        let original = committee_at(7);
        let (conflicting, _) = Committee::new_simple_test_committee_of_size(5);
        let conflicting = Committee::new(7, conflicting.voting_rights.iter().cloned().collect());
        cache.store(key(chain_identifier, 7), &original).await.unwrap();

        let error = cache.store(key(chain_identifier, 7), &conflicting).await.unwrap_err();

        assert!(matches!(error, CommitteeCacheError::Conflict { epoch: 7 }));
        assert_eq!(cache.committee(key(chain_identifier, 7)).await.unwrap(), Some(original));
        assert_eq!(cache.len().await, 1);
    }

    #[tokio::test]
    async fn clones_share_cached_committees() {
        let cache = MemoryCommitteeCache::new();
        let clone = cache.clone();
        let chain_identifier = chain_identifier(1);
        let committee = committee_at(7);

        cache.store(key(chain_identifier, 7), &committee).await.unwrap();

        assert_eq!(
            clone.committee(key(chain_identifier, 7)).await.unwrap(),
            Some(committee)
        );
    }

    #[tokio::test]
    async fn the_same_epoch_is_isolated_between_networks() {
        let cache = MemoryCommitteeCache::new();
        let first_chain = chain_identifier(1);
        let second_chain = chain_identifier(2);
        let committee = committee_at(7);

        cache
            .store(key(first_chain, committee.epoch), &committee)
            .await
            .unwrap();

        assert_eq!(
            cache.committee(key(first_chain, committee.epoch)).await.unwrap(),
            Some(committee.clone())
        );
        assert!(
            cache
                .committee(key(second_chain, committee.epoch))
                .await
                .unwrap()
                .is_none()
        );
    }
}
