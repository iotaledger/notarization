// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use serde::{Deserialize, Serialize};

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
    pub timestamp: u64,
    pub has_initial_record: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditTrailDeleted {
    pub trail_id: ObjectID,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordAdded {
    pub trail_id: ObjectID,
    pub sequence_number: u64,
    pub added_by: IotaAddress,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordDeleted {
    pub trail_id: ObjectID,
    pub sequence_number: u64,
    pub deleted_by: IotaAddress,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityIssued {
    pub security_vault_id: ObjectID,
    pub capability_id: ObjectID,
    pub role: String,
    pub issued_to: Option<IotaAddress>,
    pub valid_from: Option<u64>,
    pub valid_until: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDestroyed {
    pub security_vault_id: ObjectID,
    pub capability_id: ObjectID,
    pub role: String,
    pub issued_to: Option<IotaAddress>,
    pub valid_from: Option<u64>,
    pub valid_until: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRevoked {
    pub security_vault_id: ObjectID,
    pub capability_id: ObjectID,
}
