// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::collection_types::LinkedTable;
use iota_interaction::types::id::UID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use iota_interaction::{MoveType, ident_str};
use serde::{Deserialize, Serialize};

use super::permission::Permission;
use crate::core::internal::move_collections::{deserialize_vec_map, deserialize_vec_set};
use crate::core::internal::tx;
use crate::error::Error;

/// On-chain role and capability configuration for a trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMap {
    /// Trail object ID that this role map protects.
    pub target_key: ObjectID,
    /// Role definitions keyed by role name.
    #[serde(deserialize_with = "deserialize_vec_map")]
    pub roles: HashMap<String, Role>,
    /// Reserved role name used for initial-admin capabilities.
    pub initial_admin_role_name: String,
    /// Denylist of revoked capability IDs.
    pub revoked_capabilities: LinkedTable<ObjectID>,
    /// Capability IDs currently recognized as initial-admin capabilities.
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub initial_admin_cap_ids: HashSet<ObjectID>,
    /// Permissions required to administer roles.
    pub role_admin_permissions: RoleAdminPermissions,
    /// Permissions required to administer capabilities.
    pub capability_admin_permissions: CapabilityAdminPermissions,
}

/// Role definition stored in the trail role map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// Permissions granted by the role.
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub permissions: HashSet<Permission>,
    /// Optional role-scoped record-tag restrictions.
    pub data: Option<RoleTags>,
}

/// Defines the permissions required to administer roles in this RoleMap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAdminPermissions {
    /// Permission required to create roles.
    pub add: Permission,
    /// Permission required to delete roles.
    pub delete: Permission,
    /// Permission required to update roles.
    pub update: Permission,
}

/// Defines the permissions required to administer capabilities in this RoleMap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAdminPermissions {
    /// Permission required to issue capabilities.
    pub add: Permission,
    /// Permission required to revoke capabilities.
    pub revoke: Permission,
}

/// Capability issuance options used by the role-based API.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityIssueOptions {
    /// Address that should own the capability, if any.
    pub issued_to: Option<IotaAddress>,
    /// Millisecond timestamp at which the capability becomes valid.
    pub valid_from_ms: Option<u64>,
    /// Millisecond timestamp at which the capability expires.
    pub valid_until_ms: Option<u64>,
}

/// Allowlisted record tags stored as role data on the Move side.
///
/// The Rust name stays `RecordTags` for API continuity, but it maps to the
/// Move `record_tags::RoleTags` type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RoleTags {
    /// Allowlisted record tags for the role.
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub tags: HashSet<String>,
}

impl RoleTags {
    /// Creates role-tag restrictions from an iterator of tag names.
    pub fn new<I, S>(tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tags: tags.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns `true` when the given tag is allowed for the role.
    pub fn allows(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    pub(crate) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!("{package_id}::record_tags::RoleTags")).expect("invalid TypeTag for RoleTags")
    }

    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let mut tags = self.tags.iter().cloned().collect::<Vec<_>>();
        tags.sort();
        let tags_arg = tx::ptb_pure(ptb, "tags", tags)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("record_tags").into(),
            ident_str!("new_role_tags").into(),
            vec![],
            vec![tags_arg],
        ))
    }
}

/// Capability data returned by the Move capability module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// Capability object ID.
    pub id: UID,
    /// Trail object ID protected by the capability.
    pub target_key: ObjectID,
    /// Role granted by the capability.
    pub role: String,
    /// Capability holder, if the capability is assigned to an address.
    pub issued_to: Option<IotaAddress>,
    /// Millisecond timestamp at which the capability becomes valid.
    pub valid_from: Option<u64>,
    /// Millisecond timestamp at which the capability expires.
    pub valid_until: Option<u64>,
}

impl Capability {
    pub(crate) fn type_tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(format!("{package_id}::capability::Capability").as_str()).expect("failed to create type tag")
    }

    pub(crate) fn matches_target_and_role(&self, trail_id: ObjectID, valid_roles: &HashSet<String>) -> bool {
        self.target_key == trail_id && valid_roles.contains(&self.role)
    }
}

impl MoveType for Capability {
    fn move_type(package: ObjectID) -> TypeTag {
        Self::type_tag(package)
    }
}
