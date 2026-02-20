use std::collections::HashMap;
use std::str::FromStr;

use iota_interaction::ident_str;
// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0
use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::collection_types::LinkedTable;
use iota_interaction::types::id::UID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use serde::{Deserialize, Serialize};

use super::locking::LockingConfig;
use super::role_map::RoleMap;
use crate::core::utils;
use crate::error::Error;

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

/// Metadata set at trail creation and never updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImmutableMetadata {
    pub name: String,
    pub description: Option<String>,
}

impl ImmutableMetadata {
    pub fn new(name: String, description: Option<String>) -> Self {
        Self { name, description }
    }

    pub(in crate::core) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!("{package_id}::main::ImmutableMetadata"))
            .expect("invalid TypeTag for ImmutableMetadata")
    }

    /// Creates a new `Argument` from the `ImmutableMetadata`.
    ///
    /// To be used when creating a new `ImmutableMetadata` object on the ledger.
    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let name = utils::ptb_pure(ptb, "name", &self.name)?;
        let description = utils::ptb_pure(ptb, "description", &self.description)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("main").into(),
            ident_str!("new_trail_metadata").into(),
            vec![],
            vec![name, description],
        ))
    }
}
