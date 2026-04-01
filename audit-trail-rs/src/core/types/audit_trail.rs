// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::str::FromStr;

use iota_interaction::ident_str;
use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::collection_types::LinkedTable;
use iota_interaction::types::id::UID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use serde::{Deserialize, Serialize};

use super::locking::LockingConfig;
use super::role_map::RoleMap;
use crate::core::internal::move_collections::deserialize_vec_map;
use crate::core::internal::tx;
use crate::error::Error;

/// Registry of trail-owned record tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagRegistry {
    /// Mapping from tag name to usage count.
    #[serde(deserialize_with = "deserialize_vec_map")]
    pub tag_map: HashMap<String, u64>,
}

impl TagRegistry {
    /// Returns the number of registered tags.
    pub fn len(&self) -> usize {
        self.tag_map.len()
    }

    /// Returns `true` when no tags are registered.
    pub fn is_empty(&self) -> bool {
        self.tag_map.is_empty()
    }

    /// Returns `true` when the registry contains the given tag.
    pub fn contains_key(&self, tag: &str) -> bool {
        self.tag_map.contains_key(tag)
    }

    /// Returns the usage count for a tag.
    pub fn get(&self, tag: &str) -> Option<&u64> {
        self.tag_map.get(tag)
    }

    /// Iterates over tag names and usage counts.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &u64)> {
        self.tag_map.iter()
    }
}

/// An audit trail stored on-chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnChainAuditTrail {
    /// Unique object ID of the trail.
    pub id: UID,
    /// Address that created the trail.
    pub creator: IotaAddress,
    /// Millisecond timestamp at which the trail was created.
    pub created_at: u64,
    /// Current record sequence number cursor.
    pub sequence_number: u64,
    /// Linked table containing the trail records.
    pub records: LinkedTable<u64>,
    /// Registry of allowed record tags.
    pub tags: TagRegistry,
    /// Active locking rules for the trail.
    pub locking_config: LockingConfig,
    /// Role and capability configuration for the trail.
    pub roles: RoleMap,
    /// Metadata fixed at creation time.
    pub immutable_metadata: Option<ImmutableMetadata>,
    /// Metadata that can be updated after creation.
    pub updatable_metadata: Option<String>,
    /// On-chain package version of the trail object.
    pub version: u64,
}

/// Metadata set at trail creation and never updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImmutableMetadata {
    /// Human-readable trail name.
    pub name: String,
    /// Optional human-readable description.
    pub description: Option<String>,
}

impl ImmutableMetadata {
    /// Creates immutable metadata for a trail.
    pub fn new(name: String, description: Option<String>) -> Self {
        Self { name, description }
    }

    pub(in crate::core) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!("{package_id}::main::ImmutableMetadata"))
            .expect("invalid TypeTag for ImmutableMetadata")
    }

    /// Creates a new `Argument` from the `ImmutableMetadata`.
    ///
    /// To be used when creating a new `ImmutableMetadata` object on the ledger.
    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let name = tx::ptb_pure(ptb, "name", &self.name)?;
        let description = tx::ptb_pure(ptb, "description", &self.description)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("main").into(),
            ident_str!("new_trail_metadata").into(),
            vec![],
            vec![name, description],
        ))
    }
}
