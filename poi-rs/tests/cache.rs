// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use poi_rs::{CommitteeCache, MemoryCommitteeCache};

fn accepts_cache_trait_object(_cache: &dyn CommitteeCache) {}

#[tokio::test]
async fn memory_cache_starts_empty() {
    let cache = MemoryCommitteeCache::new();

    accepts_cache_trait_object(&cache);
    assert!(cache.is_empty().await);
    assert_eq!(cache.len().await, 0);
    assert!(cache.committee(0).await.unwrap().is_none());
}
