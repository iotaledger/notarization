// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Persist Authenticated Committees in a File-Based Cache
//!
//! A genesis-anchored verifier must authenticate every committee transition up
//! to a proof's epoch. The default cache is in memory, so its contents disappear
//! when the process exits. This advanced example supplies a file-based cache that
//! lets later runs resume from committees authenticated during earlier runs.
//!
//! Cache entries are scoped automatically to the verifier's network,
//! checked against their requested epoch, and never overwritten by a
//! conflicting value.

use std::time::Instant;

use anyhow::{Context, Result};
use file_committee_cache::FileCommitteeCache;
use poi_examples::prepare_poi_example;
use poi_rs::CommitteeResolution;

/// Demonstrates how to:
/// 1. Create a network-scoped file cache.
/// 2. Supply it to genesis-anchored committee resolution.
/// 3. Persist only committees authenticated by the resolver.
/// 4. Reuse those committees when the example is run again.
#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Proof of Inclusion Advanced: File-Based Committee Cache ===\n");

    let context = prepare_poi_example().await?;
    let cache = FileCommitteeCache::new(context.poi_dir()?.join("committee-cache"));
    let cache_directory = cache.directory().to_owned();
    let genesis = context.load_genesis().await?;
    let resolution = CommitteeResolution::from_genesis_with_cache(genesis, cache)
        .context("failed to configure genesis-anchored resolution with the file cache")?;
    let transaction_digest = context
        .create_notarization("PoI file-cache example")
        .await?
        .transaction_digest;
    let client = &context.poi_client;
    let proof = client
        .proof()
        .transaction(transaction_digest)
        .build()
        .await
        .context("failed to construct the transaction proof")?;

    println!("Network:            {}", context.network_alias);
    println!("Transaction target: {transaction_digest}");
    println!("Checkpoint epoch:   {}", proof.checkpoint_summary().epoch());
    println!("Committee cache:    {}\n", cache_directory.display());

    let verifier = client.verifier(resolution);

    println!("Verifying the proof...");
    let started = Instant::now();
    let verified = verifier
        .verify(&proof)
        .await
        .context("transaction proof verification failed")?;
    let elapsed = started.elapsed();

    println!("\nTransaction proof verified successfully in {elapsed:?}.");
    println!("  authenticated checkpoint: {}", verified.checkpoint_sequence_number());
    println!("Run the example again to reuse the authenticated committees stored on disk.");

    Ok(())
}

mod file_committee_cache {

    use std::fmt::Write as _;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    use iota_types::committee::Committee;
    use poi_rs::{CommitteeCache, CommitteeCacheError, CommitteeCacheKey};
    use tempfile::NamedTempFile;

    /// File-backed storage for committees authenticated by a resolver.
    ///
    /// Each network and epoch is stored in a separate BCS file.
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

        fn network_directory(&self, key: CommitteeCacheKey) -> PathBuf {
            let digest = key.chain_identifier().as_bytes();
            let mut chain = String::with_capacity(digest.len() * 2);
            for byte in digest {
                write!(&mut chain, "{byte:02x}").expect("writing to a string cannot fail");
            }

            self.directory.join(chain)
        }

        fn committee_path(&self, key: CommitteeCacheKey) -> PathBuf {
            self.network_directory(key).join(format!("epoch-{}.bcs", key.epoch()))
        }

        fn backend(
            key: CommitteeCacheKey,
            source: impl std::error::Error + Send + Sync + 'static,
        ) -> CommitteeCacheError {
            CommitteeCacheError::Backend {
                epoch: key.epoch(),
                source: Box::new(source),
            }
        }
    }

    #[async_trait::async_trait]
    impl CommitteeCache for FileCommitteeCache {
        async fn committee(&self, key: CommitteeCacheKey) -> Result<Option<Committee>, CommitteeCacheError> {
            let path = self.committee_path(key);
            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(error) => return Err(Self::backend(key, error)),
            };
            let committee: Committee = bcs::from_bytes(&bytes).map_err(|error| Self::backend(key, error))?;

            if committee.epoch != key.epoch() {
                return Err(CommitteeCacheError::Conflict { epoch: key.epoch() });
            }

            Ok(Some(committee))
        }

        async fn store(&self, key: CommitteeCacheKey, committee: &Committee) -> Result<(), CommitteeCacheError> {
            let epoch = committee.epoch;
            if key.epoch() != epoch {
                return Err(CommitteeCacheError::Conflict { epoch });
            }

            if let Some(cached) = self.committee(key).await? {
                return if cached == *committee {
                    Ok(())
                } else {
                    Err(CommitteeCacheError::Conflict { epoch })
                };
            }

            let directory = self.network_directory(key);
            fs::create_dir_all(&directory).map_err(|error| Self::backend(key, error))?;
            let bytes = bcs::to_bytes(committee).map_err(|error| Self::backend(key, error))?;
            let mut temporary = NamedTempFile::new_in(&directory).map_err(|error| Self::backend(key, error))?;
            temporary
                .write_all(&bytes)
                .and_then(|()| temporary.as_file().sync_all())
                .map_err(|error| Self::backend(key, error))?;

            match temporary.persist_noclobber(self.committee_path(key)) {
                Ok(_) => Ok(()),
                Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                    match self.committee(key).await? {
                        Some(cached) if cached == *committee => Ok(()),
                        _ => Err(CommitteeCacheError::Conflict { epoch }),
                    }
                }
                Err(error) => Err(Self::backend(key, error.error)),
            }
        }
    }
}
