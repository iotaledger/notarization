// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_interaction::types::base_types::IotaAddress;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use iota_interaction::types::{MOVE_STDLIB_PACKAGE_ID, TypeTag};
use serde::{Deserialize, Deserializer, Serialize};

use crate::core::utils;
use crate::error::Error;

use std::collections::{HashMap, HashSet};

/// Page of records loaded through linked-table traversal.
#[derive(Debug, Clone)]
pub struct PaginatedRecord<D = Data> {
    pub records: HashMap<u64, Record<D>>,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Data {
    Bytes(Vec<u8>),
    Text(String),
}

impl<'de> Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Handle both raw bytes and string representations from BCS
        let bytes = Vec::<u8>::deserialize(deserializer)?;

        if let Ok(text) = String::from_utf8(bytes.clone()) {
            // Additional check: if it looks like actual text (not just valid UTF-8 bytes)
            if text.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()) {
                Ok(Data::Text(text))
            } else {
                Ok(Data::Bytes(bytes))
            }
        } else {
            Ok(Data::Bytes(bytes))
        }
    }
}

impl Data {
    /// Returns the Move type tag for this data type.
    pub(crate) fn tag(&self) -> TypeTag {
        match self {
            Data::Bytes(_) => TypeTag::Vector(Box::new(TypeTag::U8)),
            Data::Text(_) => TypeTag::from_str(&format!("{MOVE_STDLIB_PACKAGE_ID}::string::String"))
                .expect("should be valid type tag"),
        }
    }

    /// Creates a PTB argument for `D` where `D` is the concrete Move data type.
    pub(in crate::core) fn to_ptb(self, ptb: &mut Ptb, name: &str) -> Result<Argument, Error> {
        match self {
            Data::Bytes(bytes) => utils::ptb_pure(ptb, name, bytes),
            Data::Text(text) => utils::ptb_pure(ptb, name, text),
        }
    }

    /// Creates a PTB argument for `Option<D>` where `D` is the concrete Move data type.
    pub(in crate::core) fn to_option_ptb(self, ptb: &mut Ptb, name: &str) -> Result<Argument, Error> {
        match self {
            Data::Bytes(bytes) => utils::ptb_pure(ptb, name, Some(bytes)),
            Data::Text(text) => utils::ptb_pure(ptb, name, Some(text)),
        }
    }

    /// Validates that this data payload matches the on-chain trail data type.
    pub(in crate::core) fn ensure_matches_tag(&self, expected: &TypeTag) -> Result<(), Error> {
        let actual = self.tag();

        if &actual == expected {
            Ok(())
        } else {
            Err(Error::InvalidArgument(format!(
                "record data type mismatch: provided {:?}, trail expects {:?}",
                actual, expected
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

#[cfg(test)]
mod tests {
    use super::Data;

    fn deserialize_from_raw_bytes(payload: Vec<u8>) -> Data {
        let encoded = bcs::to_bytes(&payload).expect("failed to bcs encode bytes payload");
        bcs::from_bytes::<Data>(&encoded).expect("failed to deserialize Data from bcs payload")
    }

    #[test]
    fn deserialize_ascii_text_returns_text_variant() {
        let data = deserialize_from_raw_bytes(b"hello world".to_vec());
        assert_eq!(data, Data::Text("hello world".to_string()));
    }

    #[test]
    fn deserialize_ascii_text_with_whitespace_returns_text_variant() {
        let data = deserialize_from_raw_bytes(b"line 1\nline 2\tend".to_vec());
        assert_eq!(data, Data::Text("line 1\nline 2\tend".to_string()));
    }

    #[test]
    fn deserialize_non_ascii_utf8_returns_bytes_variant() {
        let data = deserialize_from_raw_bytes("olá mundo".as_bytes().to_vec());
        assert_eq!(data, Data::Bytes("olá mundo".as_bytes().to_vec()));
    }

    #[test]
    fn deserialize_ascii_like_binary_returns_text_variant() {
        // Demonstrates current heuristic limitation: printable ASCII payloads are interpreted as text.
        let data = deserialize_from_raw_bytes(b"GIF89a".to_vec());
        assert_eq!(data, Data::Text("GIF89a".to_string()));
    }

    #[test]
    fn deserialize_utf8_with_control_chars_returns_bytes_variant() {
        let data = deserialize_from_raw_bytes(vec![b'a', b'b', 0x00, b'c']);
        assert_eq!(data, Data::Bytes(vec![b'a', b'b', 0x00, b'c']));
    }

    #[test]
    fn deserialize_invalid_utf8_returns_bytes_variant() {
        let data = deserialize_from_raw_bytes(vec![0xF0, 0x28, 0x8C, 0x28]);
        assert_eq!(data, Data::Bytes(vec![0xF0, 0x28, 0x8C, 0x28]));
    }

    #[test]
    fn deserialize_empty_payload_returns_empty_text() {
        let data = deserialize_from_raw_bytes(Vec::new());
        assert_eq!(data, Data::Text(String::new()));
    }
}
