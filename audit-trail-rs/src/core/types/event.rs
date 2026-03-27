// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleCreated {
    #[serde(rename = "target_key")]
    pub trail_id: ObjectID,
    pub role: String,
    #[serde(deserialize_with = "deserialize_permission_set")]
    pub permissions: PermissionSet,
    pub data: Option<RoleTags>,
    pub created_by: IotaAddress,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleUpdated {
    #[serde(rename = "target_key")]
    pub trail_id: ObjectID,
    pub role: String,
    #[serde(rename = "new_permissions", deserialize_with = "deserialize_permission_set")]
    pub permissions: PermissionSet,
    #[serde(rename = "new_data")]
    pub data: Option<RoleTags>,
    pub updated_by: IotaAddress,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleDeleted {
    #[serde(rename = "target_key")]
    pub trail_id: ObjectID,
    pub role: String,
    pub deleted_by: IotaAddress,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

fn deserialize_permission_set<'de, D>(deserializer: D) -> Result<PermissionSet, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let vec_set = EventVecSet::<EventPermission>::deserialize(deserializer)?;
    let permissions = vec_set
        .contents
        .into_iter()
        .map(|permission| permission.into_permission())
        .collect::<Result<_, _>>()?;

    Ok(PermissionSet { permissions })
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct EventVecSet<T> {
    contents: Vec<T>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct EventPermission {
    variant: String,
}

impl EventPermission {
    fn into_permission<E>(self) -> Result<Permission, E>
    where
        E: serde::de::Error,
    {
        match self.variant.as_str() {
            "DeleteAuditTrail" => Ok(Permission::DeleteAuditTrail),
            "DeleteAllRecords" => Ok(Permission::DeleteAllRecords),
            "AddRecord" => Ok(Permission::AddRecord),
            "DeleteRecord" => Ok(Permission::DeleteRecord),
            "CorrectRecord" => Ok(Permission::CorrectRecord),
            "UpdateLockingConfig" => Ok(Permission::UpdateLockingConfig),
            "UpdateLockingConfigForDeleteRecord" => Ok(Permission::UpdateLockingConfigForDeleteRecord),
            "UpdateLockingConfigForDeleteTrail" => Ok(Permission::UpdateLockingConfigForDeleteTrail),
            "UpdateLockingConfigForWrite" => Ok(Permission::UpdateLockingConfigForWrite),
            "AddRoles" => Ok(Permission::AddRoles),
            "UpdateRoles" => Ok(Permission::UpdateRoles),
            "DeleteRoles" => Ok(Permission::DeleteRoles),
            "AddCapabilities" => Ok(Permission::AddCapabilities),
            "RevokeCapabilities" => Ok(Permission::RevokeCapabilities),
            "UpdateMetadata" => Ok(Permission::UpdateMetadata),
            "DeleteMetadata" => Ok(Permission::DeleteMetadata),
            "Migrate" => Ok(Permission::Migrate),
            "AddRecordTags" => Ok(Permission::AddRecordTags),
            "DeleteRecordTags" => Ok(Permission::DeleteRecordTags),
            other => Err(E::custom(format!(
                "unknown permission variant `{other}` in event payload"
            ))),
        }
    }
}
