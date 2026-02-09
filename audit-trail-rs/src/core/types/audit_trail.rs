use std::collections::HashMap;

// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0
use iota_interaction::types::base_types::IotaAddress;
use iota_interaction::types::collection_types::LinkedTable;
use iota_interaction::types::id::UID;
use serde::{Deserialize, Serialize};

use crate::core::types::Data;

use super::locking::LockingConfig;
use super::metadata::ImmutableMetadata;
use super::record::Record;
use super::role_map::RoleMap;

/// An audit trail stored on-chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnChainAuditTrail {
    pub id: UID,
    pub creator: IotaAddress,
    pub created_at: u64,
    pub sequence_number: u64,
    pub records: LinkedTable<u64>,
    pub locking_config: LockingConfig,
    pub roles: RoleMap,
    pub immutable_metadata: Option<ImmutableMetadata>,
    pub updatable_metadata: Option<String>,
    pub version: u64,
}
