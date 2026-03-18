// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::id::UID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use iota_interaction::{MoveType, ident_str};
use serde::{Deserialize, Serialize};

use super::permission::Permission;
use crate::core::utils;
use crate::core::utils::{deserialize_vec_map, deserialize_vec_set};
use crate::error::Error;
use crate::package;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMap {
    pub target_key: ObjectID,
    #[serde(deserialize_with = "deserialize_vec_map")]
    pub roles: HashMap<String, Role>,
    pub initial_admin_role_name: String,
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub issued_capabilities: HashSet<ObjectID>,
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub initial_admin_cap_ids: HashSet<ObjectID>,
    pub role_admin_permissions: RoleAdminPermissions,
    pub capability_admin_permissions: CapabilityAdminPermissions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub permissions: HashSet<Permission>,
    pub data: Option<RecordTags>,
}

/// Defines the permissions required to administer roles in this RoleMap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAdminPermissions {
    pub add: Permission,
    pub delete: Permission,
    pub update: Permission,
}

/// Defines the permissions required to administer capabilities in this RoleMap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAdminPermissions {
    pub add: Permission,
    pub revoke: Permission,
}

/// Capability issuance options used by the role-based API.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityIssueOptions {
    pub issued_to: Option<IotaAddress>,
    pub valid_from_ms: Option<u64>,
    pub valid_until_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RecordTags {
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub allowed_tags: HashSet<String>,
}

impl RecordTags {
    pub fn new<I, S>(allowed_tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            allowed_tags: allowed_tags.into_iter().map(Into::into).collect(),
        }
    }

    pub fn allows(&self, tag: &str) -> bool {
        self.allowed_tags.contains(tag)
    }

    pub(crate) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!("{package_id}::record_tags::RecordTags")).expect("invalid TypeTag for RecordTags")
    }

    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let mut allowed_tags = self.allowed_tags.iter().cloned().collect::<Vec<_>>();
        allowed_tags.sort();
        let allowed_tags_arg = utils::ptb_pure(ptb, "allowed_tags", allowed_tags)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("record_tags").into(),
            ident_str!("new_record_tags").into(),
            vec![],
            vec![allowed_tags_arg],
        ))
    }
}

/// Capability data returned by the Move capability module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub id: UID,
    pub target_key: ObjectID,
    pub role: String,
    pub issued_to: Option<IotaAddress>,
    pub valid_from: Option<u64>,
    pub valid_until: Option<u64>,
}

impl MoveType for Capability {
    fn move_type(_: ObjectID) -> TypeTag {
        let object_id = package::tf_components_package_id();
        TypeTag::from_str(format!("{object_id}::capability::Capability").as_str()).expect("failed to create type tag")
    }
}
