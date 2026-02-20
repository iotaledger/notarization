// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use iota_interaction::MoveType;
use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::id::UID;
use serde::{Deserialize, Serialize};

use super::permission::Permission;
use crate::core::utils::{deserialize_vec_map, deserialize_vec_set};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMap {
    pub target_key: ObjectID,
    #[serde(deserialize_with = "deserialize_vec_map")]
    pub roles: HashMap<String, HashSet<Permission>>,
    pub initial_admin_role_name: String,
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub issued_capabilities: HashSet<ObjectID>,
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub initial_admin_cap_ids: HashSet<ObjectID>,
    pub role_admin_permissions: RoleAdminPermissions,
    pub capability_admin_permissions: CapabilityAdminPermissions,
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
    fn move_type(package: ObjectID) -> TypeTag {
        TypeTag::from_str(format!("{package}::capability::Capability").as_str()).expect("failed to create type tag")
    }
}
