// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Notarization State
//!
//! This module defines the state structure for notarizations, representing the actual data
//! being attested on the blockchain.
//!
//! ## Overview
//!
//! The state consists of two components:
//! - **Data**: The primary content being notarized (bytes or text)
//! - **Metadata**: Optional descriptive information about the data
//!
//! ## Data Types
//!
//! The module supports two data formats:
//! - **Bytes**: Raw binary data for files, images, or serialized objects
//! - **Text**: UTF-8 encoded strings for documents or structured data
//!
//! ## Examples
//!
//! ### Creating State from Text
//!
//! ```rust
//! use notarization::core::types::State;
//!
//! let state = State::from_string(
//!     "Contract Agreement v2.1".to_string(),
//!     Some("Legal document".to_string()),
//! );
//! ```
//!
//! ### Creating State from Bytes
//!
//! ```rust
//! use notarization::core::types::State;
//!
//! let pdf_content = vec![0x25, 0x50, 0x44, 0x46]; // PDF header
//! let state = State::from_bytes(pdf_content, Some("Signed contract PDF".to_string()));
//! ```

use std::str::FromStr;

use iota_interaction::ident_str;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::Argument;
use iota_interaction::types::{MOVE_STDLIB_PACKAGE_ID, TypeTag};
use serde::{Deserialize, Deserializer, Serialize};

use super::super::move_utils;
use crate::error::Error;

/// Represents the state of a notarization.
///
/// State encapsulates the data being notarized along with optional metadata.
/// It serves as the primary content container for both locked and dynamic
/// notarizations.
///
/// ## Type Parameter
///
/// - `T`: The data type, defaults to [`Data`] which can be either bytes or text
#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct State<T = Data> {
    /// The actual data being notarized
    pub data: T,
    /// Optional metadata describing the data
    #[serde(default)]
    pub metadata: Option<String>,
}

/// Represents the different types of data that can be notarized.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Data {
    /// Raw binary data (e.g., files, images, serialized objects)
    Bytes(Vec<u8>),
    /// UTF-8 text data (e.g., documents, JSON, configuration)
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
    ///
    /// Used internally for blockchain transaction construction.
    pub(crate) fn tag(&self) -> TypeTag {
        match self {
            Data::Bytes(_) => TypeTag::Vector(Box::new(TypeTag::U8)),
            Data::Text(_) => TypeTag::from_str(&format!("{MOVE_STDLIB_PACKAGE_ID}::string::String"))
                .expect("should be valid type tag"),
        }
    }

    /// Extracts the data as bytes.
    ///
    /// ## Errors
    ///
    /// Returns an error if the data is text rather than bytes.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use notarization::core::types::{State, Data};
    /// # use notarization::error::Error;
    /// let state = State::from_bytes(vec![1, 2, 3], None);
    /// let bytes = state.data.as_bytes()?;
    /// assert_eq!(bytes, vec![1, 2, 3]);
    /// # Ok::<(), Error>(())
    /// ```
    pub fn as_bytes(self) -> Result<Vec<u8>, Error> {
        match self {
            Data::Bytes(data) => Ok(data),
            Data::Text(_) => Err(Error::GenericError("Data is not a vector".to_string())),
        }
    }

    /// Extracts the data as text.
    ///
    /// ## Errors
    ///
    /// Returns an error if the data is bytes rather than text.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # use notarization::core::types::{State, Data};
    /// # use notarization::error::Error;
    /// let state = State::from_string("Hello".to_string(), None);
    /// let text = state.data.as_text()?;
    /// assert_eq!(text, "Hello");
    /// # Ok::<(), Error>(())
    /// ```
    pub fn as_text(self) -> Result<String, Error> {
        match self {
            Data::Bytes(_) => Err(Error::GenericError("Data is not a string".to_string())),
            Data::Text(data) => Ok(data),
        }
    }
}

impl State {
    /// Returns a reference to the data.
    pub fn data(&self) -> &Data {
        &self.data
    }

    /// Returns a reference to the metadata.
    pub fn metadata(&self) -> &Option<String> {
        &self.metadata
    }

    /// Creates a new state from raw bytes.
    ///
    /// Use this for binary data like files, images, or serialized content.
    ///
    /// ## Parameters
    ///
    /// - `data`: The raw bytes to store
    /// - `metadata`: Optional description of the bytes
    ///
    /// ## Example
    ///
    /// ```rust
    /// use notarization::core::types::State;
    ///
    /// // Store a file's content
    /// let file_bytes = vec![0xFF, 0xD8, 0xFF];
    /// let state = State::from_bytes(file_bytes, Some("Profile photo JPEG".to_string()));
    /// ```
    pub fn from_bytes(data: Vec<u8>, metadata: Option<String>) -> Self {
        Self {
            data: Data::Bytes(data),
            metadata,
        }
    }

    /// Creates a new state from a string.
    ///
    /// Use this for text data like documents, JSON, or configuration.
    ///
    /// ## Parameters
    ///
    /// - `data`: The text content to store
    /// - `metadata`: Optional description of the text
    ///
    /// ## Example
    ///
    /// ```rust
    /// use notarization::core::types::State;
    ///
    /// // Store a JSON configuration
    /// let config = r#"{"version": "1.0", "enabled": true}"#;
    /// let state = State::from_string(
    ///     config.to_string(),
    ///     Some("Service configuration".to_string()),
    /// );
    /// ```
    pub fn from_string(data: String, metadata: Option<String>) -> Self {
        Self {
            data: Data::Text(data),
            metadata,
        }
    }

    /// Creates a new `Argument` from the `State`.
    ///
    /// To be used when creating a new `Notarization` object on the ledger.
    pub(in crate::core) fn into_ptb(
        self,
        ptb: &mut ProgrammableTransactionBuilder,
        package_id: ObjectID,
    ) -> Result<Argument, Error> {
        match self.data {
            Data::Bytes(data) => state_from_bytes(ptb, data, self.metadata, package_id),
            Data::Text(data) => state_from_string(ptb, data, self.metadata, package_id),
        }
    }
}

/// Helper function to create a new state from bytes.
fn state_from_bytes(
    ptb: &mut ProgrammableTransactionBuilder,
    data: Vec<u8>,
    metadata: Option<String>,
    package_id: ObjectID,
) -> Result<Argument, Error> {
    let data = move_utils::ptb_pure(ptb, "data", data)?;
    let metadata = move_utils::ptb_pure(ptb, "metadata", metadata)?;

    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("notarization").into(),
        ident_str!("new_state_from_bytes").into(),
        vec![],
        vec![data, metadata],
    ))
}

/// Helper function to create a new state from bytes.
fn state_from_string(
    ptb: &mut ProgrammableTransactionBuilder,
    data: String,
    metadata: Option<String>,
    package_id: ObjectID,
) -> Result<Argument, Error> {
    let data = move_utils::ptb_pure(ptb, "data", data)?;
    let metadata = move_utils::ptb_pure(ptb, "metadata", metadata)?;

    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("notarization").into(),
        ident_str!("new_state_from_string").into(),
        vec![],
        vec![data, metadata],
    ))
}
