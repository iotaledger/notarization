// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::collection_types::VecSet;
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::{deserialize_number_from_string, deserialize_option_number_from_string};

use super::{Permission, PermissionSet, RoleTags};

/// Generic wrapper for audit trail events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event<D> {
    /// Parsed event payload.
    #[serde(flatten)]
    pub data: D,
}

/// Event emitted when a trail is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditTrailCreated {
    /// Newly created trail object ID.
    pub trail_id: ObjectID,
    /// Address that created the trail.
    pub creator: IotaAddress,
    /// Millisecond event timestamp.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

/// Event emitted when a trail is deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditTrailDeleted {
    /// Deleted trail object ID.
    pub trail_id: ObjectID,
    /// Millisecond event timestamp.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

/// Event emitted when a record is added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordAdded {
    /// Trail object ID receiving the new record.
    pub trail_id: ObjectID,
    /// Sequence number assigned to the new record.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub sequence_number: u64,
    /// Address that added the record.
    pub added_by: IotaAddress,
    /// Millisecond event timestamp.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

/// Event emitted when a record is deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordDeleted {
    /// Trail object ID from which the record was deleted.
    pub trail_id: ObjectID,
    /// Sequence number of the deleted record.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub sequence_number: u64,
    /// Address that deleted the record.
    pub deleted_by: IotaAddress,
    /// Millisecond event timestamp.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

/// Event emitted when a capability is issued.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityIssued {
    /// Trail object ID protected by the capability.
    pub target_key: ObjectID,
    /// Newly created capability object ID.
    pub capability_id: ObjectID,
    /// Role granted by the capability.
    pub role: String,
    /// Address receiving the capability, if one is assigned.
    pub issued_to: Option<IotaAddress>,
    /// Millisecond timestamp at which the capability becomes valid.
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_from: Option<u64>,
    /// Millisecond timestamp at which the capability expires.
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_until: Option<u64>,
}

/// Event emitted when a capability object is destroyed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDestroyed {
    /// Trail object ID protected by the capability.
    pub target_key: ObjectID,
    /// Destroyed capability object ID.
    pub capability_id: ObjectID,
    /// Role granted by the capability.
    pub role: String,
    /// Address that held the capability, if any.
    pub issued_to: Option<IotaAddress>,
    /// Millisecond timestamp at which the capability became valid.
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_from: Option<u64>,
    /// Millisecond timestamp at which the capability expired.
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_until: Option<u64>,
}

/// Event emitted when a capability is revoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRevoked {
    /// Trail object ID protected by the capability.
    pub target_key: ObjectID,
    /// Revoked capability object ID.
    pub capability_id: ObjectID,
    /// Millisecond timestamp retained for denylist cleanup.
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub valid_until: u64,
}

/// Event emitted when a role is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleCreated {
    /// Trail object ID that owns the role.
    pub trail_id: ObjectID,
    /// Role name.
    pub role: String,
    /// Permissions granted by the new role.
    pub permissions: PermissionSet,
    /// Optional record-tag restrictions stored as role data.
    pub data: Option<RoleTags>,
    /// Address that created the role.
    pub created_by: IotaAddress,
    /// Millisecond event timestamp.
    pub timestamp: u64,
}

/// Event emitted when a role is updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleUpdated {
    /// Trail object ID that owns the role.
    pub trail_id: ObjectID,
    /// Role name.
    pub role: String,
    /// Updated permissions for the role.
    pub permissions: PermissionSet,
    /// Updated record-tag restrictions, if any.
    pub data: Option<RoleTags>,
    /// Address that updated the role.
    pub updated_by: IotaAddress,
    /// Millisecond event timestamp.
    pub timestamp: u64,
}

/// Event emitted when a role is deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleDeleted {
    /// Trail object ID that owned the role.
    pub trail_id: ObjectID,
    /// Role name.
    pub role: String,
    /// Address that deleted the role.
    pub deleted_by: IotaAddress,
    /// Millisecond event timestamp.
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
