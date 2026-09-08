// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::{Committee, EpochId};
use iota_types::digests::ChainIdentifier;

use crate::BoxError;

mod in_memory;

pub use in_memory::MemoryCommitteeCache;

/// Identifies one committee-cache entry by its network and epoch.
///
/// The chain identifier is the network's genesis checkpoint digest. Cache
/// adapters must use the complete key so entries authenticated for different
/// networks cannot collide.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CommitteeCacheKey {
    chain_identifier: ChainIdentifier,
    epoch: EpochId,
}

impl CommitteeCacheKey {
    /// Creates a cache key for `epoch` on the identified network.
    pub const fn new(chain_identifier: ChainIdentifier, epoch: EpochId) -> Self {
        Self {
            chain_identifier,
            epoch,
        }
    }

    /// Creates a key for a cache private to one resolver.
    pub(crate) fn isolated(epoch: EpochId) -> Self {
        Self::new(ChainIdentifier::default(), epoch)
    }

    /// Returns the network's genesis checkpoint digest.
    pub const fn chain_identifier(&self) -> &ChainIdentifier {
        &self.chain_identifier
    }

    /// Returns the cached committee epoch.
    pub const fn epoch(&self) -> EpochId {
        self.epoch
    }
}

/// Error returned by a committee cache.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CommitteeCacheError {
    /// A cached committee conflicts with authenticated committee data.
    #[error("cached committee conflicts at epoch {epoch}")]
    Conflict {
        /// Epoch whose cached material conflicts.
        epoch: EpochId,
    },
    /// A cache backend failed to read or write committee material.
    #[error("committee cache backend failed at epoch {epoch}")]
    Backend {
        /// Epoch being accessed when the backend failed.
        epoch: EpochId,
        /// Underlying backend error.
        #[source]
        source: BoxError,
    },
}

/// Stores authenticated committees for anchored resolution.
///
/// Implementations must preserve committee integrity after storage and use the
/// complete [`CommitteeCacheKey`] for every lookup.
#[async_trait::async_trait]
pub trait CommitteeCache: Send + Sync {
    /// Returns the authenticated committee for `key`, when available.
    async fn committee(&self, key: CommitteeCacheKey) -> Result<Option<Committee>, CommitteeCacheError>;

    /// Stores a committee under `key` after the resolver has authenticated it.
    async fn store(&self, key: CommitteeCacheKey, committee: &Committee) -> Result<(), CommitteeCacheError>;
}
