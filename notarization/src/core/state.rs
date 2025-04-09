// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use identity_iota_interaction::ident_str;
use iota_sdk::types::base_types::ObjectID;
use iota_sdk::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_sdk::types::transaction::Argument;
use iota_sdk::types::{TypeTag, MOVE_STDLIB_PACKAGE_ID};

use crate::error::Error;

use super::utils;

/// The state of the `Notarization` that can be updated
pub struct State {
    pub data: Data,
    metadata: Option<String>,
}

pub enum Data {
    Vector(Vec<u8>),
    Text(String),
}

impl Data {
    pub(crate) fn tag(&self) -> TypeTag {
        match self {
            Data::Vector(_) => TypeTag::Vector(Box::new(TypeTag::U8)),
            Data::Text(_) => {
                TypeTag::from_str(&format!("{MOVE_STDLIB_PACKAGE_ID}::string::String"))
                    .expect("could not create string tag")
            }
        }
    }
}

impl State {
    pub fn new(data: Data, metadata: Option<String>) -> Self {
        Self { data, metadata }
    }

    pub fn data(&self) -> &Data {
        &self.data
    }

    pub fn metadata(&self) -> &Option<String> {
        &self.metadata
    }

    pub fn new_from_vector(data: Vec<u8>, metadata: Option<String>) -> Self {
        Self {
            data: Data::Vector(data),
            metadata,
        }
    }

    pub fn new_from_string(data: String, metadata: Option<String>) -> Self {
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
            Data::Vector(data) => new_from_vector(ptb, data, self.metadata, package_id),
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
    let data = utils::ptb_pure(ptb, "data", data)?;
    let metadata = utils::new_move_option_string(metadata, ptb)?;

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
    let data = utils::new_move_string(data, ptb)?;
    let metadata = utils::new_move_option_string(metadata, ptb)?;

    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("notarization").into(),
        ident_str!("new_from_string").into(),
        vec![],
        vec![data, metadata],
    ))
}
