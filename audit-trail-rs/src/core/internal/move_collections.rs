// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use iota_interaction::types::collection_types::{VecMap, VecSet};
use serde::{Deserialize, Deserializer};

pub(crate) fn deserialize_vec_map<'de, D, K, V>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Eq + Hash + Debug,
    V: Deserialize<'de> + Debug,
{
    let vec_map = VecMap::<K, V>::deserialize(deserializer)?;
    Ok(vec_map
        .contents
        .into_iter()
        .map(|entry| (entry.key, entry.value))
        .collect())
}

pub(crate) fn deserialize_vec_set<'de, D, T>(deserializer: D) -> Result<HashSet<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Eq + Hash,
{
    let vec_set = VecSet::<T>::deserialize(deserializer)?;
    Ok(vec_set.contents.into_iter().collect())
}
