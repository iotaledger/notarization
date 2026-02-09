// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::id::UID;
use iota_interaction::types::TypeTag;
use iota_interaction::MoveType;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

impl MoveType for Capability {
    fn move_type(package: ObjectID) -> TypeTag {
        TypeTag::from_str(format!("{package}::capability::Capability").as_str())
            .expect("failed to create type tag")
    }
}
