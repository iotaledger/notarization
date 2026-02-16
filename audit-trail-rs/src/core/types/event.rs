// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::{deserialize_number_from_string, deserialize_option_number_from_string};
use std::collections::HashSet;

use crate::core::utils::deserialize_vec_set;

use super::permission::Permission;
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleCreated {
    pub trail_id: ObjectID,
    pub role: String,
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub permissions: HashSet<Permission>,
    pub created_by: IotaAddress,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleUpdated {
    pub trail_id: ObjectID,
    pub role: String,
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub new_permissions: HashSet<Permission>,
    pub updated_by: IotaAddress,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleDeleted {
    pub trail_id: ObjectID,
    pub role: String,
    pub deleted_by: IotaAddress,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub timestamp: u64,
}
