// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_interaction::ident_str;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::Argument;
use iota_interaction::types::{TypeTag, MOVE_STDLIB_PACKAGE_ID};
use serde::{Deserialize, Serialize};

use super::move_utils;
use crate::error::Error;

/// The state of the `Notarization` that can be updated
#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct State<T = Data> {
    pub data: T,
    #[serde(default)]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Data {
    Bytes(Vec<u8>),
    Text(String),
}

impl Data {
    pub(crate) fn tag(&self) -> TypeTag {
        match self {
            Data::Bytes(_) => TypeTag::Vector(Box::new(TypeTag::U8)),
            Data::Text(_) => TypeTag::from_str(&format!("{MOVE_STDLIB_PACKAGE_ID}::string::String"))
                .expect("should be valid type tag"),
        }
    }
}

impl State {
    pub fn data(&self) -> &Data {
        &self.data
    }

    pub fn metadata(&self) -> &Option<String> {
        &self.metadata
    }

    pub fn from_bytes(data: Vec<u8>, metadata: Option<String>) -> Self {
        Self {
            data: Data::Bytes(data),
            metadata,
        }
    }

    pub fn from_string(data: String, metadata: Option<String>) -> Self {
        Self {
            data: Data::Text(data),
            metadata,
        }
    }

    /// Creates a new `Argument` from the `State`.
    ///
    /// To be used when creating a new `Notarization` object on the ledger.
    pub(super) fn into_ptb(
        self,
        ptb: &mut ProgrammableTransactionBuilder,
        package_id: ObjectID,
    ) -> Result<Argument, Error> {
        match self.data {
            Data::Bytes(data) => new_from_vector(ptb, data, self.metadata, package_id),
            Data::Text(data) => new_from_string(ptb, data, self.metadata, package_id),
        }
    }
}

pub(crate) fn new_from_vector(
    ptb: &mut ProgrammableTransactionBuilder,
    data: Vec<u8>,
    metadata: Option<String>,
    package_id: ObjectID,
) -> Result<Argument, Error> {
    let data = move_utils::ptb_pure(ptb, "data", data)?;
    let metadata = move_utils::new_move_option_string(metadata, ptb)?;

    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("notarization").into(),
        ident_str!("new_from_vector").into(),
        vec![],
        vec![data, metadata],
    ))
}

pub(crate) fn new_from_string(
    ptb: &mut ProgrammableTransactionBuilder,
    data: String,
    metadata: Option<String>,
    package_id: ObjectID,
) -> Result<Argument, Error> {
    let data = move_utils::new_move_string(data, ptb)?;
    let metadata = move_utils::new_move_option_string(metadata, ptb)?;

    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("notarization").into(),
        ident_str!("new_from_string").into(),
        vec![],
        vec![data, metadata],
    ))
}
