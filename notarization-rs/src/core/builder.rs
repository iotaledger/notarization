// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # Notarization Builder
//!
//! The builder module provides a type-safe, fluent API for creating notarizations on the IOTA blockchain.
//!
//! ## Overview
//!
//! A notarization is a blockchain-based attestation that creates tamper-proof records with timestamps
//! and version tracking. This module supports two types of notarizations:
//!
//! - **Locked**: Immutable records that cannot be modified after creation
//! - **Dynamic**: Updatable records that can evolve over time
//!
//! ## Examples
//!
//! ### Creating a Locked Notarization
//!
//! ```rust,ignore
//! use notarization::core::builder::NotarizationBuilder;
//! use notarization::core::types::{State, TimeLock};
//!
//! let builder = NotarizationBuilder::locked()
//!     .with_string_state("Legal Document v1.0".to_string(), Some("PDF hash: abc123".to_string()))
//!     .with_immutable_description("Employment Contract".to_string())
//!     .with_delete_at(TimeLock::UnlockAt(1735689600)) // Unix timestamp
//!     .finish()?;
//! ```
//!
//! ### Creating a Dynamic Notarization
//!
//! ```rust,ignore
//! use notarization::core::builder::NotarizationBuilder;
//! use notarization::core::types::{State, TimeLock};
//!
//! let builder = NotarizationBuilder::dynamic()
//!     .with_string_state("Status: Active".to_string(), None)
//!     .with_immutable_description("Service Status Monitor".to_string())
//!     .with_transfer_lock(TimeLock::None) // Can be transferred freely
//!     .finish();
//! ```

use std::marker::PhantomData;

use product_common::transaction::transaction_builder::TransactionBuilder;

use super::transactions::CreateNotarization;
use super::types::{NotarizationMethod, State, TimeLock};
use crate::error::Error;

/// Marker type for locked notarizations.
#[derive(Clone)]
pub struct Locked;

/// Marker type for dynamic notarizations.
#[derive(Clone)]
pub struct Dynamic;

/// A builder for constructing notarization transactions.
///
/// This builder uses the type parameter `M` to enforce method-specific
/// constraints
/// at compile time. The two supported types are [`NotarizationMethod::Locked`] and [`NotarizationMethod::Dynamic`].
#[derive(Debug, Clone)]
pub struct NotarizationBuilder<M> {
    /// The data to be notarized
    pub state: Option<State>,
    /// A permanent description set at creation
    pub immutable_description: Option<String>,
    /// Metadata that can be updated
    pub updatable_metadata: Option<String>,
    /// Time restriction for deletion (Locked only)
    pub delete_lock: Option<TimeLock>,
    /// Time restriction for transfers (Dynamic only)
    pub transfer_lock: Option<TimeLock>,
    /// The notarization method
    pub method: NotarizationMethod,
    _marker: PhantomData<M>,
}

impl NotarizationBuilder<Locked> {
    /// Creates a new builder for a locked notarization.
    ///
    /// Locked notarizations are immutable after creation. They cannot be updated
    /// or transferred, and can only be destroyed after a specified time period.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use notarization::core::builder::NotarizationBuilder;
    /// use notarization::core::types::TimeLock;
    ///
    /// let builder = NotarizationBuilder::locked().with_delete_at(TimeLock::UnlockAt(1735689600));
    /// ```
    pub fn locked() -> Self {
        Self {
            state: None,
            immutable_description: None,
            updatable_metadata: None,
            delete_lock: None,
            transfer_lock: None,
            method: NotarizationMethod::Locked,
            _marker: PhantomData,
        }
    }

    /// Sets when the notarization can be destroyed.
    ///
    /// This is required for locked notarizations. Common patterns:
    /// - `TimeLock::UnlockAt(timestamp)`: Can be destroyed after specific time
    /// - `TimeLock::UntilDestroyed`: Can never be destroyed (permanent record)
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use notarization::core::builder::NotarizationBuilder;
    /// use notarization::core::types::TimeLock;
    ///
    /// // Can be destroyed after January 1, 2025
    /// let builder = NotarizationBuilder::locked().with_delete_at(TimeLock::UnlockAt(1735689600));
    /// ```
    pub fn with_delete_lock(mut self, lock: TimeLock) -> Self {
        self.delete_lock = Some(lock);
        self
    }

    /// Finalizes the builder and creates a transaction builder.
    ///
    /// ## Errors
    ///
    /// Returns an error if `delete_lock` is not set, as it's required for locked notarizations.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// # use notarization::core::builder::NotarizationBuilder;
    /// # use notarization::core::types::{TimeLock, State};
    /// let transaction = NotarizationBuilder::locked()
    ///     .with_string_state("Document content", None)
    ///     .with_delete_lock(TimeLock::UnlockAt(1735689600))
    ///     .finish()?;
    /// # Ok::<(), notarization::Error>(())
    /// ```
    pub fn finish(self) -> Result<TransactionBuilder<CreateNotarization<Locked>>, Error> {
        Ok(TransactionBuilder::new(CreateNotarization::new(self)))
    }
}

impl NotarizationBuilder<Dynamic> {
    /// Creates a new builder for a dynamic notarization.
    ///
    /// Dynamic notarizations can be updated after creation and optionally
    /// transferred to other owners. They maintain a version counter that
    /// increments with each update.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use notarization::core::builder::NotarizationBuilder;
    ///
    /// let builder = NotarizationBuilder::dynamic().with_string_state("Initial state", None);
    /// ```
    pub fn dynamic() -> Self {
        Self {
            state: None,
            immutable_description: None,
            updatable_metadata: None,
            delete_lock: None,
            transfer_lock: None,
            method: NotarizationMethod::Dynamic,
            _marker: PhantomData,
        }
    }

    /// Sets restrictions on when the notarization can be transferred.
    ///
    /// By default, dynamic notarizations can be transferred freely. Use this
    /// to add time-based restrictions.
    ///
    /// ## Common Patterns
    ///
    /// - `TimeLock::None`: Can be transferred anytime (default)
    /// - `TimeLock::UnlockAt(timestamp)`: Can be transferred after specific time
    /// - `TimeLock::UntilDestroyed`: Can never be transferred
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use notarization::core::builder::NotarizationBuilder;
    /// use notarization::core::types::TimeLock;
    ///
    /// // Lock transfers for 30 days
    /// let builder = NotarizationBuilder::dynamic().with_transfer_lock(TimeLock::UnlockAt(1735689600));
    /// ```
    pub fn with_transfer_lock(mut self, lock: TimeLock) -> Self {
        self.transfer_lock = Some(lock);
        self
    }

    /// Finalizes the builder and creates a transaction builder.
    ///
    /// Unlike locked notarizations, dynamic notarizations have no required fields
    /// beyond the method type.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// # use notarization::core::builder::NotarizationBuilder;
    /// # use notarization::core::types::State;
    /// let transaction = NotarizationBuilder::dynamic()
    ///     .with_string_state("Dynamic content", None)
    ///     .with_immutable_description("Status Monitor")
    ///     .finish();
    /// ```
    pub fn finish(self) -> TransactionBuilder<CreateNotarization<Dynamic>> {
        TransactionBuilder::new(CreateNotarization::new(self))
    }
}

// Shared methods for both types
impl<M> NotarizationBuilder<M> {
    /// Sets the state (data) to be notarized.
    ///
    /// The state is the actual content being notarized. It can contain any data
    /// along with optional metadata.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use notarization::core::builder::NotarizationBuilder;
    /// use notarization::core::types::State;
    ///
    /// let builder = NotarizationBuilder::locked()
    ///     .with_state(State::from_string("Document content", Some("v1.0")));
    /// ```
    pub fn with_state(mut self, state: State) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets the state using raw bytes.
    ///
    /// Convenience method for binary data like file contents or serialized objects.
    ///
    /// ## Parameters
    ///
    /// - `data`: The raw bytes to notarize
    /// - `metadata`: Optional metadata about the bytes (e.g., "PDF document", "Image file")
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use notarization::core::builder::NotarizationBuilder;
    ///
    /// let pdf_bytes = vec![0x25, 0x50, 0x44, 0x46]; // PDF header
    /// let builder =
    ///     NotarizationBuilder::locked().with_bytes_state(pdf_bytes, Some("Contract PDF".to_string()));
    /// ```
    pub fn with_bytes_state(self, data: Vec<u8>, metadata: Option<String>) -> Self {
        self.with_state(State::from_bytes(data, metadata))
    }

    /// Sets the state using a string.
    ///
    /// Convenience method for text-based data.
    ///
    /// ## Parameters
    ///
    /// - `data`: The string content to notarize
    /// - `metadata`: Optional metadata (e.g., "JSON config", "Status message")
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use notarization::core::builder::NotarizationBuilder;
    ///
    /// let builder = NotarizationBuilder::dynamic().with_string_state(
    ///     r#"{"status": "active", "version": "2.0"}"#.to_string(),
    ///     Some("Service configuration".to_string()),
    /// );
    /// ```
    pub fn with_string_state(self, data: String, metadata: Option<String>) -> Self {
        self.with_state(State::from_string(data, metadata))
    }

    /// Sets a permanent description for the notarization.
    ///
    /// This description is immutable and cannot be changed after creation.
    /// Use it to provide context about what is being notarized.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use notarization::core::builder::NotarizationBuilder;
    ///
    /// let builder = NotarizationBuilder::locked()
    ///     .with_immutable_description("Q4 2024 Financial Report".to_string());
    /// ```
    pub fn with_immutable_description(mut self, description: String) -> Self {
        self.immutable_description = Some(description);
        self
    }

    /// Sets initial updatable metadata.
    ///
    /// Unlike the immutable description, this metadata can be updated later
    /// (for dynamic notarizations only).
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use notarization::core::builder::NotarizationBuilder;
    ///
    /// let builder =
    ///     NotarizationBuilder::dynamic().with_updatable_metadata("Status: Draft".to_string());
    /// ```
    pub fn with_updatable_metadata(mut self, metadata: String) -> Self {
        self.updatable_metadata = Some(metadata);
        self
    }
}
