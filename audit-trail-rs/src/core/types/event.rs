// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_sdk::types::collection_types::VecSet;
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::{deserialize_number_from_string, deserialize_option_number_from_string};

use super::{Permission, PermissionSet, RoleTags};

/// Generic wrapper for audit trail events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event<D> {
    #[serde(flatten)]
    pub data: D,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditTrailCreated {
    pub trail_id: ObjectID,
    pub creator: IotaAddress,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditTrailDeleted {
    pub trail_id: ObjectID,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordAdded {
    pub trail_id: ObjectID,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub sequence_number: u64,
    pub added_by: IotaAddress,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordDeleted {
    pub trail_id: ObjectID,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub sequence_number: u64,
    pub deleted_by: IotaAddress,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityIssued {
    pub target_key: ObjectID,
    pub capability_id: ObjectID,
    pub role: String,
    pub issued_to: Option<IotaAddress>,
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_from: Option<u64>,
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_until: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDestroyed {
    pub target_key: ObjectID,
    pub capability_id: ObjectID,
    pub role: String,
    pub issued_to: Option<IotaAddress>,
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_from: Option<u64>,
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_until: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRevoked {
    pub target_key: ObjectID,
    pub capability_id: ObjectID,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub valid_until: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleCreated {
    pub trail_id: ObjectID,
    pub role: String,
    pub permissions: PermissionSet,
    pub data: Option<RoleTags>,
    pub created_by: IotaAddress,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleUpdated {
    pub trail_id: ObjectID,
    pub role: String,
    pub permissions: PermissionSet,
    pub data: Option<RoleTags>,
    pub updated_by: IotaAddress,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleDeleted {
    pub trail_id: ObjectID,
    pub role: String,
    pub deleted_by: IotaAddress,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct RawRoleCreated {
    target_key: ObjectID,
    role: String,
    permissions: VecSet<Permission>,
    data: Option<RawRoleTags>,
    created_by: IotaAddress,
    timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct RawRoleUpdated {
    target_key: ObjectID,
    role: String,
    new_permissions: VecSet<Permission>,
    new_data: Option<RawRoleTags>,
    updated_by: IotaAddress,
    timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct RawRoleDeleted {
    target_key: ObjectID,
    role: String,
    deleted_by: IotaAddress,
    timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct RawRoleTags {
    tags: VecSet<String>,
}

impl From<VecSet<Permission>> for PermissionSet {
    fn from(value: VecSet<Permission>) -> Self {
        Self {
            permissions: value.contents.into_iter().collect::<HashSet<_>>(),
        }
    }
}

impl From<RawRoleTags> for RoleTags {
    fn from(value: RawRoleTags) -> Self {
        Self {
            tags: value.tags.contents.into_iter().collect::<HashSet<_>>(),
        }
    }
}

impl From<RawRoleCreated> for RoleCreated {
    fn from(value: RawRoleCreated) -> Self {
        Self {
            trail_id: value.target_key,
            role: value.role,
            permissions: value.permissions.into(),
            data: value.data.map(Into::into),
            created_by: value.created_by,
            timestamp: value.timestamp,
        }
    }
}

impl From<RawRoleUpdated> for RoleUpdated {
    fn from(value: RawRoleUpdated) -> Self {
        Self {
            trail_id: value.target_key,
            role: value.role,
            permissions: value.new_permissions.into(),
            data: value.new_data.map(Into::into),
            updated_by: value.updated_by,
            timestamp: value.timestamp,
        }
    }
}

impl From<RawRoleDeleted> for RoleDeleted {
    fn from(value: RawRoleDeleted) -> Self {
        Self {
            trail_id: value.target_key,
            role: value.role,
            deleted_by: value.deleted_by,
            timestamp: value.timestamp,
        }
    }
}
