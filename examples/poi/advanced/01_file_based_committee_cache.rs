// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Persist Authenticated Committees in a File-Based Cache
//!
//! A genesis-anchored verifier must authenticate every committee transition up
//! to a proof's epoch. The default cache is in memory, so its contents disappear
//! when the process exits. This advanced example supplies a file-based cache that
//! lets later runs resume from committees authenticated during earlier runs.
//!
//! The cache is part of the verifier's trust boundary. Its directory is scoped
//! to mainnet, cached values are checked against their requested epoch, and an
//! authenticated committee can never overwrite a conflicting cached value.

use std::time::Instant;

use anyhow::{Context, Result};
use iota_sdk_types::TransactionDigest;
use poi_rs::{CommitteeResolution, PoiClient};

#[path = "../utils.rs"]
mod utils;

use file_committee_cache::FileCommitteeCache;
use utils::{load_mainnet_genesis, mainnet_poi_dir};

const MAINNET_TRANSACTION_DIGEST: &str = "86EvhdjqBb6Rt5pB8sKjTnE7MrzpNLuWTH3SuELBjDvu";

/// Demonstrates how to:
/// 1. Create a network-scoped file cache.
/// 2. Supply it to genesis-anchored committee resolution.
/// 3. Persist only committees authenticated by the resolver.
/// 4. Reuse those committees when the example is run again.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion Advanced: File-Based Committee Cache ===\n");

    let transaction_digest = MAINNET_TRANSACTION_DIGEST
        .parse::<TransactionDigest>()
        .context("the example transaction digest must be valid")?;
    let client = PoiClient::mainnet().context("failed to configure the public mainnet gRPC endpoint")?;
    let proof = client
        .proof()
        .transaction(transaction_digest)
        .build()
        .await
        .context("failed to construct the transaction proof")?;

    let cache = FileCommitteeCache::new(mainnet_poi_dir()?.join("committee-cache"));
    println!("Transaction target: {transaction_digest}");
    println!("Checkpoint epoch:   {}", proof.checkpoint_summary.epoch());
    println!("Committee cache:    {}\n", cache.directory().display());

    let genesis = load_mainnet_genesis().await?;
    let resolution = CommitteeResolution::from_genesis_with_cache(genesis, cache)
        .context("failed to configure genesis-anchored resolution with the file cache")?;
    let verifier = client.verifier(resolution);

    println!("Verifying the proof...");
    let started = Instant::now();
    verifier
        .verify(&proof)
        .await
        .context("transaction proof verification failed")?;
    let elapsed = started.elapsed();

    println!("\nTransaction proof verified successfully in {elapsed:?}.");
    println!("Run the example again to reuse the authenticated committees stored on disk.");

    Ok(())
}

mod file_committee_cache {

    use std::{fs, io::Write, path::PathBuf};

    use iota_types::committee::{Committee, EpochId};
    use poi_rs::{CommitteeCache, CommitteeCacheError};
    use tempfile::NamedTempFile;

    /// File-backed storage for committees authenticated by a resolver.
    ///
    /// Each epoch is stored in a separate BCS file. The directory must be scoped to
    /// one trusted network and genesis anchor; mixing networks would violate the
    /// cache's trust contract.
    #[derive(Clone, Debug)]
    pub struct FileCommitteeCache {
        directory: PathBuf,
    }

    impl FileCommitteeCache {
        pub const fn new(directory: PathBuf) -> Self {
            Self { directory }
        }

        pub fn directory(&self) -> &PathBuf {
            &self.directory
        }

        fn committee_path(&self, epoch: EpochId) -> PathBuf {
            self.directory.join(format!("epoch-{epoch}.bcs"))
        }

        fn backend(epoch: EpochId, source: impl std::error::Error + Send + Sync + 'static) -> CommitteeCacheError {
            CommitteeCacheError::Backend {
                epoch,
                source: Box::new(source),
            }
        }
    }

    #[async_trait::async_trait]
    impl CommitteeCache for FileCommitteeCache {
        async fn committee(&self, epoch: EpochId) -> Result<Option<Committee>, CommitteeCacheError> {
            let path = self.committee_path(epoch);
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(Self::backend(epoch, error)),
            };
            let committee: Committee = bcs::from_bytes(&bytes).map_err(|error| Self::backend(epoch, error))?;

            if committee.epoch != epoch {
                return Err(CommitteeCacheError::Conflict { epoch });
            }

            Ok(Some(committee))
        }

        async fn store(&self, committee: &Committee) -> Result<(), CommitteeCacheError> {
            let epoch = committee.epoch;

            if let Some(cached) = self.committee(epoch).await? {
                return if cached == *committee {
                    Ok(())
                } else {
                    Err(CommitteeCacheError::Conflict { epoch })
                };
            }

            fs::create_dir_all(&self.directory).map_err(|error| Self::backend(epoch, error))?;
            let bytes = bcs::to_bytes(committee).map_err(|error| Self::backend(epoch, error))?;
            let mut temporary = NamedTempFile::new_in(&self.directory).map_err(|error| Self::backend(epoch, error))?;
            temporary
                .write_all(&bytes)
                .and_then(|()| temporary.as_file().sync_all())
                .map_err(|error| Self::backend(epoch, error))?;

            match temporary.persist_noclobber(self.committee_path(epoch)) {
                Ok(_) => Ok(()),
                Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                    match self.committee(epoch).await? {
                        Some(cached) if cached == *committee => Ok(()),
                        _ => Err(CommitteeCacheError::Conflict { epoch }),
                    }
                }
                Err(error) => Err(Self::backend(epoch, error.error)),
            }
        }
    }
}
