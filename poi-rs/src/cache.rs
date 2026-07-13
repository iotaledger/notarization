// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::{Committee, EpochId};

use crate::BoxError;

mod in_memory;

pub use in_memory::MemoryCommitteeCache;

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
/// A cache is part of the caller's trust boundary. Implementations must return
/// only committees previously authenticated for the same network and must
/// preserve their integrity after storage.
#[async_trait::async_trait]
pub trait CommitteeCache: Send + Sync {
    /// Returns the authenticated committee for `epoch`, when available.
    async fn committee(&self, epoch: EpochId) -> Result<Option<Committee>, CommitteeCacheError>;

    /// Stores a committee after the resolver has authenticated it.
    async fn store(&self, committee: &Committee) -> Result<(), CommitteeCacheError>;
}
