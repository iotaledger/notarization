// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

use iota_interaction::ident_str;
use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use serde::{Deserialize, Serialize};

use crate::core::internal::tx;
use crate::error::Error;

/// Page of records loaded through linked-table traversal.
#[derive(Debug, Clone)]
pub struct PaginatedRecord<D = Data> {
    /// Records included in the current page, keyed by sequence number.
    pub records: BTreeMap<u64, Record<D>>,
    /// Cursor to pass to the next [`TrailRecords::list_page`](crate::core::records::TrailRecords::list_page) call.
    pub next_cursor: Option<u64>,
    /// Indicates whether another page may be available.
    pub has_next_page: bool,
}

/// A single record in the audit trail.
///
/// Records form a tamper-evident, sequential chain: each record receives a monotonically increasing
/// sequence number that is never reused, even after the record is deleted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record<D = Data> {
    /// Record payload stored on-chain.
    pub data: D,
    /// Optional application-defined metadata.
    pub metadata: Option<String>,
    /// Optional trail-owned tag attached to the record.
    pub tag: Option<String>,
    /// Monotonic record sequence number inside the trail.
    pub sequence_number: u64,
    /// Address that added the record.
    pub added_by: IotaAddress,
    /// Millisecond timestamp at which the record was added.
    pub added_at: u64,
    /// Correction relationships for this record.
    pub correction: RecordCorrection,
}

/// Input used when creating a trail with an initial record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitialRecord<D = Data> {
    /// Initial payload to store in the trail.
    pub data: D,
    /// Optional application-defined metadata.
    pub metadata: Option<String>,
    /// Optional initial tag from the trail-owned registry.
    pub tag: Option<String>,
}

impl InitialRecord {
    /// Creates a new initial record.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use audit_trail::core::types::{Data, InitialRecord};
    ///
    /// let record = InitialRecord::new(
    ///     Data::text("hello"),
    ///     Some("seed".to_string()),
    ///     Some("inbox".to_string()),
    /// );
    ///
    /// assert_eq!(record.data, Data::text("hello"));
    /// assert_eq!(record.metadata.as_deref(), Some("seed"));
    /// assert_eq!(record.tag.as_deref(), Some("inbox"));
    /// ```
    pub fn new(data: impl Into<Data>, metadata: Option<String>, tag: Option<String>) -> Self {
        Self {
            data: data.into(),
            metadata,
            tag,
        }
    }

    pub(crate) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!(
            "{package_id}::record::InitialRecord<{}>",
            Data::tag(package_id)
        ))
        .expect("invalid TypeTag for InitialRecord")
    }

    pub(in crate::core) fn into_ptb(self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let data_tag = Data::tag(package_id);
        let data = self.data.into_ptb(ptb, package_id)?;
        let metadata = tx::ptb_pure(ptb, "initial_record_metadata", self.metadata)?;
        let tag = tx::ptb_pure(ptb, "initial_record_tag", self.tag)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("record").into(),
            ident_str!("new_initial_record").into(),
            vec![data_tag],
            vec![data, metadata, tag],
        ))
    }
}

/// Bidirectional correction tracking for audit records.
///
/// `replaces` is fixed at creation and lists the sequence numbers this record supersedes;
/// `is_replaced_by` is a back-pointer the trail sets later when *this* record itself is corrected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RecordCorrection {
    /// Sequence numbers that this record supersedes.
    pub replaces: HashSet<u64>,
    /// Sequence number of the record that supersedes this one, if any.
    pub is_replaced_by: Option<u64>,
}

impl RecordCorrection {
    /// Creates a correction value that replaces the given sequence numbers.
    pub fn with_replaces(replaces: HashSet<u64>) -> Self {
        Self {
            replaces,
            is_replaced_by: None,
        }
    }

    /// Returns `true` when this record supersedes at least one earlier record.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashSet;
    ///
    /// use audit_trail::core::types::RecordCorrection;
    ///
    /// let correction = RecordCorrection::with_replaces(HashSet::from([1, 2]));
    ///
    /// assert!(correction.is_correction());
    /// ```
    pub fn is_correction(&self) -> bool {
        !self.replaces.is_empty()
    }

    /// Returns `true` when this record has itself been replaced by a later record.
    pub fn is_replaced(&self) -> bool {
        self.is_replaced_by.is_some()
    }
}

/// Supported record data types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Data {
    /// Arbitrary binary payload.
    Bytes(Vec<u8>),
    /// UTF-8 text payload.
    Text(String),
}

impl Data {
    /// Returns the Move type tag for `record::Data`.
    pub(crate) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!("{package_id}::record::Data")).expect("should be valid type tag")
    }

    /// Creates a PTB argument for `record::Data`.
    pub(in crate::core) fn into_ptb(self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        match self {
            Data::Bytes(bytes) => {
                let bytes = tx::ptb_pure(ptb, "data_bytes", bytes)?;
                Ok(ptb.programmable_move_call(
                    package_id,
                    ident_str!("record").into(),
                    ident_str!("new_bytes").into(),
                    vec![],
                    vec![bytes],
                ))
            }
            Data::Text(text) => {
                let text = tx::ptb_pure(ptb, "data_text", text)?;
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

    /// Validates that the on-chain trail stores `record::Data`.
    pub(in crate::core) fn ensure_matches_tag(&self, expected: &TypeTag, package_id: ObjectID) -> Result<(), Error> {
        let actual = Self::tag(package_id);

        if &actual == expected {
            Ok(())
        } else {
            Err(Error::InvalidArgument(format!(
                "record data type mismatch: trail expects {:?}, SDK writes {:?}",
                expected, actual
            )))
        }
    }

    /// Creates a new `Data` from bytes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use audit_trail::core::types::Data;
    ///
    /// assert_eq!(Data::bytes([1_u8, 2, 3]), Data::Bytes(vec![1, 2, 3]));
    /// ```
    pub fn bytes(data: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(data.into())
    }

    /// Creates a new `Data` from text.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use audit_trail::core::types::Data;
    ///
    /// assert_eq!(Data::text("hello"), Data::Text("hello".to_string()));
    /// ```
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

    #[test]
    fn data_bcs_roundtrip_preserves_text_variant() {
        let encoded = bcs::to_bytes(&Data::Text("hello world".to_string())).expect("failed to encode Data");
        let data = bcs::from_bytes::<Data>(&encoded).expect("failed to decode Data");
        assert_eq!(data, Data::Text("hello world".to_string()));
    }

    #[test]
    fn data_bcs_roundtrip_preserves_bytes_variant() {
        let encoded =
            bcs::to_bytes(&Data::Bytes(vec![0x47, 0x49, 0x46, 0x38, 0x39, 0x61])).expect("failed to encode Data");
        let data = bcs::from_bytes::<Data>(&encoded).expect("failed to decode Data");
        assert_eq!(data, Data::Bytes(vec![0x47, 0x49, 0x46, 0x38, 0x39, 0x61]));
    }
}
