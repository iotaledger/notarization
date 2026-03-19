// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

use iota_interaction::ident_str;
use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::IotaAddress;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use serde::{Deserialize, Serialize};

use crate::core::utils;
use crate::error::Error;

/// Page of records loaded through linked-table traversal.
#[derive(Debug, Clone)]
pub struct PaginatedRecord<D = Data> {
    pub records: BTreeMap<u64, Record<D>>,
    pub next_cursor: Option<u64>,
    pub has_next_page: bool,
}

/// A single record in the audit trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record<D = Data> {
    pub data: D,
    pub metadata: Option<String>,
    pub sequence_number: u64,
    pub added_by: IotaAddress,
    pub added_at: u64,
    pub correction: RecordCorrection,
}

/// Bidirectional correction tracking for audit records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RecordCorrection {
    pub replaces: HashSet<u64>,
    pub is_replaced_by: Option<u64>,
}

impl RecordCorrection {
    pub fn with_replaces(replaces: HashSet<u64>) -> Self {
        Self {
            replaces,
            is_replaced_by: None,
        }
    }

    pub fn is_correction(&self) -> bool {
        !self.replaces.is_empty()
    }

    pub fn is_replaced(&self) -> bool {
        self.is_replaced_by.is_some()
    }
}

/// Supported record data types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Data {
    Bytes(Vec<u8>),
    Text(String),
}

impl Data {
    pub(crate) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!("{package_id}::record::Data")).expect("should be valid type tag")
    }

    /// Creates a PTB argument for the Move `record::Data` type exposed by the Rust SDK.
    pub(in crate::core) fn to_ptb(self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        match self {
            Data::Bytes(bytes) => {
                let bytes = utils::ptb_pure(ptb, "bytes", bytes)?;
                Ok(ptb.programmable_move_call(
                    package_id,
                    ident_str!("record").into(),
                    ident_str!("new_bytes").into(),
                    vec![],
                    vec![bytes],
                ))
            }
            Data::Text(text) => {
                let text = utils::ptb_pure(ptb, "text", text)?;
                Ok(ptb.programmable_move_call(
                    package_id,
                    ident_str!("record").into(),
                    ident_str!("new_text").into(),
                    vec![],
                    vec![text],
                ))
            }
        }
    }

    /// Creates a PTB argument for `Option<record::Data>`.
    pub(in crate::core) fn to_option_ptb(self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let data = self.to_ptb(ptb, package_id)?;
        utils::option_to_move(Some(data), Self::tag(package_id), ptb)
            .map_err(|e| Error::InvalidArgument(format!("failed to build record data option: {e}")))
    }

    /// Validates that the on-chain trail stores the Move `record::Data` type supported by the Rust SDK.
    pub(in crate::core) fn ensure_supported_trail_tag(expected: &TypeTag, package_id: ObjectID) -> Result<(), Error> {
        let supported = Self::tag(package_id);

        if expected == &supported {
            Ok(())
        } else {
            Err(Error::InvalidArgument(format!(
                "unsupported trail record type {expected:?}; expected {:?}",
                supported
            )))
        }
    }

    /// Creates a new `Data` from bytes.
    pub fn bytes(data: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(data.into())
    }

    /// Creates a new `Data` from text.
    pub fn text(data: impl Into<String>) -> Self {
        Self::Text(data.into())
    }

    /// Extracts the data as bytes.
    ///
    /// ## Errors
    ///
    /// Returns an error if the data is text rather than bytes.
    pub fn as_bytes(self) -> Result<Vec<u8>, Error> {
        match self {
            Data::Bytes(data) => Ok(data),
            Data::Text(_) => Err(Error::GenericError("Data is not bytes".to_string())),
        }
    }

    /// Extracts the data as text.
    ///
    /// ## Errors
    ///
    /// Returns an error if the data is bytes rather than text.
    pub fn as_text(self) -> Result<String, Error> {
        match self {
            Data::Bytes(_) => Err(Error::GenericError("Data is not text".to_string())),
            Data::Text(data) => Ok(data),
        }
    }
}

impl From<String> for Data {
    fn from(value: String) -> Self {
        Data::Text(value)
    }
}

impl From<&str> for Data {
    fn from(value: &str) -> Self {
        Data::Text(value.to_string())
    }
}

impl From<Vec<u8>> for Data {
    fn from(value: Vec<u8>) -> Self {
        Data::Bytes(value)
    }
}

impl From<&[u8]> for Data {
    fn from(value: &[u8]) -> Self {
        Data::Bytes(value.to_vec())
    }
}
