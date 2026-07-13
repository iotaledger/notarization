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

#[cfg(test)]
mod tests {
    use std::{error::Error as _, io};

    use super::*;

    #[test]
    fn cache_is_object_safe() {
        let cache = MemoryCommitteeCache::new();

        let _: &dyn CommitteeCache = &cache;
    }

    #[test]
    fn conflict_error_identifies_the_epoch() {
        let error = CommitteeCacheError::Conflict { epoch: 7 };

        assert_eq!(error.to_string(), "cached committee conflicts at epoch 7");
        assert!(error.source().is_none());
    }

    #[test]
    fn backend_error_preserves_the_epoch_and_source() {
        let error = CommitteeCacheError::Backend {
            epoch: 11,
            source: Box::new(io::Error::other("storage unavailable")),
        };

        assert_eq!(error.to_string(), "committee cache backend failed at epoch 11");
        assert_eq!(error.source().unwrap().to_string(), "storage unavailable");
    }
}
