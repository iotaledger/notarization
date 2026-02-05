// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::id::UID;
use serde::{Deserialize, Serialize};

/// Capability data returned by the Move capability module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub id: UID,
    pub security_vault_id: ObjectID,
    pub role: String,
    pub issued_to: Option<IotaAddress>,
    pub valid_from: Option<u64>,
    pub valid_until: Option<u64>,
}
