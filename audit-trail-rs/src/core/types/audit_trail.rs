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

/// Registry of record tags configured for an audit trail.
///
/// Each entry maps a tag name to its current usage count across role definitions and records.
///
/// `TagRegistry` maintains a combined usage count per tag that is
/// incremented every time a record or role references the tag. A tag cannot be
/// removed from the registry while its usage count is greater than zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagRegistry {
    /// Mapping from a human-readable tag name to how many times it is used in role definitions and records.
    #[serde(deserialize_with = "deserialize_vec_map")]
    pub tag_map: HashMap<String, u64>,
}

impl TagRegistry {
    /// Returns the number of tags currently registered.
    pub fn len(&self) -> usize {
        self.tag_map.len()
    }

    /// Returns `true` if the registry contains no tags.
    pub fn is_empty(&self) -> bool {
        self.tag_map.is_empty()
    }

    /// Returns `true` if a tag with the given name exists in the registry.
    ///
    /// - `tag`: The tag name to look up.
    pub fn contains_key(&self, tag: &str) -> bool {
        self.tag_map.contains_key(tag)
    }

    /// Returns the current usage count associated with a tag name.
    ///
    /// - `tag`: The tag name to look up.
    pub fn get(&self, tag: &str) -> Option<&u64> {
        self.tag_map.get(tag)
    }

    /// Iterates over all registered tag names and their usage counts.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &u64)> {
        self.tag_map.iter()
    }
}

/// An audit trail stored on-chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnChainAuditTrail {
    /// Unique object id of the audit trail.
    pub id: UID,
    /// Address that originally created the trail.
    pub creator: IotaAddress,
    /// Unix timestamp in milliseconds when the trail was created.
    pub created_at: u64,
    /// Monotonically increasing number.
    /// Can be interpreted as total number of created `Record` instances by this audit trai.
    /// Will be used as identifier for the next record added to the trail, starting at 0 for the first record.
    pub sequence_number: u64,
    /// Contains the trail's records keyed by `sequence_number`.
    pub records: LinkedTable<u64>,
    /// Registry of tag names tracked with their current usage counts.
    /// Tag names can be added to records to restrict record-access to users having capabilities
    /// granting access to this tag. Tag specific access can be defined by adding tags to [`Role`] definitions.
    pub tags: TagRegistry,
    /// Active write/delete locking rules for this trail.
    pub locking_config: LockingConfig,
    /// [`Role`] definitions and permissions configured for the trail.
    pub roles: RoleMap,
    /// Immutable metadata set at creation time, if present.
    pub immutable_metadata: Option<ImmutableMetadata>,
    /// Mutable metadata string that can be updated after creation, if present.
    pub updatable_metadata: Option<String>,
    /// On-chain schema or object version maintained by the Move package.
    pub version: u64,
}

/// Metadata set at trail creation and never updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImmutableMetadata {
    /// Human-readable trail name.
    pub name: String,
    /// Optional longer description explaining the trail's purpose.
    pub description: Option<String>,
}

impl ImmutableMetadata {
    /// Creates immutable metadata for a new trail.
    ///
    /// - `name`: The human-readable name to store on the trail.
    /// - `description`: An optional longer description stored alongside the name.
    pub fn new(name: String, description: Option<String>) -> Self {
        Self { name, description }
    }

    /// Returns the Move type tag for `main::ImmutableMetadata` in the given package.
    ///
    /// - `package_id`: The published audit-trail Move package id.
    pub(in crate::core) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!("{package_id}::main::ImmutableMetadata"))
            .expect("invalid TypeTag for ImmutableMetadata")
    }

    /// Creates a new `Argument` from the `ImmutableMetadata`.
    ///
    /// To be used when creating a new `ImmutableMetadata` object on the ledger.
    ///
    /// - `ptb`: The programmable transaction builder the argument should be added to.
    /// - `package_id`: The published audit-trail Move package id.
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
