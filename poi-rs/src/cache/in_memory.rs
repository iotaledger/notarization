// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{collections::BTreeMap, sync::Arc};

use iota_types::committee::{Committee, EpochId};
use tokio::sync::RwLock;

use super::{CommitteeCache, CommitteeCacheError};

/// In-memory committee cache for library usage and tests.
#[derive(Clone, Debug, Default)]
pub struct MemoryCommitteeCache {
    committees: Arc<RwLock<BTreeMap<EpochId, Committee>>>,
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
    async fn committee(&self, epoch: EpochId) -> Result<Option<Committee>, CommitteeCacheError> {
        Ok(self.committees.read().await.get(&epoch).cloned())
    }

    async fn store(&self, committee: &Committee) -> Result<(), CommitteeCacheError> {
        let epoch = committee.epoch;
        let mut committees = self.committees.write().await;

        if committees.get(&epoch).is_some_and(|cached| cached != committee) {
            return Err(CommitteeCacheError::Conflict { epoch });
        }

        committees.entry(epoch).or_insert_with(|| committee.clone());

        Ok(())
    }
}
