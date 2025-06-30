// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction_ts::wasm_error::Result;
use js_sys::Uint8Array;
use notarization::core::builder::{Dynamic, Locked, NotarizationBuilder};
use product_common::bindings::transaction::WasmTransactionBuilder;
use wasm_bindgen::prelude::*;

use crate::wasm_notarization::{WasmCreateNotarizationDynamic, WasmCreateNotarizationLocked};
use crate::wasm_time_lock::WasmTimeLock;

/// Represents a builder for constructing locked notarization transactions.
///
/// Locked notarizations are immutable records (the notarization state) that cannot be modified
/// after creation.
#[wasm_bindgen(js_name = NotarizationBuilderLocked, inspectable)]
pub struct WasmNotarizationBuilderLocked(pub(crate) NotarizationBuilder<Locked>);

/// Converts a `NotarizationBuilder<Locked>` into a `WasmNotarizationBuilderLocked`.
impl From<NotarizationBuilder<Locked>> for WasmNotarizationBuilderLocked {
    fn from(val: NotarizationBuilder<Locked>) -> Self {
        WasmNotarizationBuilderLocked(val)
    }
}

/// Provides methods for building locked notarization transactions.
#[wasm_bindgen(js_class = NotarizationBuilderLocked)]
impl WasmNotarizationBuilderLocked {
    /// Adds a state to the notarization using binary data.
    ///
    /// # Arguments
    /// * `data` - Binary data representing the state.
    /// * `metadata` - Optional metadata associated with the state.
    #[wasm_bindgen(js_name = withBytesState)]
    pub fn with_bytes_state(self, data: Uint8Array, metadata: Option<String>) -> Self {
        self.0.with_bytes_state(data.to_vec(), metadata).into()
    }

    /// Adds a state to the notarization using a string.
    ///
    /// # Arguments
    /// * `data` - String data representing the state.
    /// * `metadata` - Optional metadata associated with the state.
    #[wasm_bindgen(js_name = withStringState)]
    pub fn with_string_state(self, data: String, metadata: Option<String>) -> Self {
        self.0.with_string_state(data, metadata).into()
    }

    /// Adds an immutable description to the notarization.
    ///
    /// # Arguments
    /// * `description` - A string describing the notarization.
    #[wasm_bindgen(js_name = withImmutableDescription)]
    pub fn with_immutable_description(self, description: String) -> Self {
        self.0.with_immutable_description(description).into()
    }

    /// Adds updatable metadata to the notarization.
    ///
    /// # Arguments
    /// * `metadata` - A string representing the metadata.
    #[wasm_bindgen(js_name = withUpdatableMetadata)]
    pub fn with_updatable_metadata(self, metadata: String) -> Self {
        self.0.with_updatable_metadata(metadata).into()
    }

    /// Creates a new locked notarization builder.
    #[wasm_bindgen()]
    pub fn locked() -> Self {
        NotarizationBuilder::<Locked>::locked().into()
    }

    /// Adds a delete lock to the notarization.
    ///
    /// # Arguments
    /// * `lock` - A `TimeLock` specifying the delete lock.
    #[wasm_bindgen(js_name = withDeleteLock)]
    pub fn with_delete_lock(self, lock: WasmTimeLock) -> Self {
        self.0.with_delete_lock(lock.0).into()
    }

    /// Finalizes the notarization builder and returns a transaction builder
    /// that can be used to build and execute the final transaction on the ledger.
    ///
    /// # Returns
    /// A `TransactionBuilder` to build and execute the transaction.
    #[wasm_bindgen()]
    pub fn finish(self) -> Result<WasmTransactionBuilder> {
        let js_value: JsValue = WasmCreateNotarizationLocked::new(self).into();
        Ok(WasmTransactionBuilder::new(js_value.unchecked_into()))
    }
}

/// Represents a builder for constructing dynamic notarization transactions.
///
/// Dynamic notarizations are updatable records that can evolve over time.
#[wasm_bindgen(js_name = NotarizationBuilderDynamic)]
pub struct WasmNotarizationBuilderDynamic(pub(crate) NotarizationBuilder<Dynamic>);

impl From<NotarizationBuilder<Dynamic>> for WasmNotarizationBuilderDynamic {
    fn from(val: NotarizationBuilder<Dynamic>) -> Self {
        WasmNotarizationBuilderDynamic(val)
    }
}

/// Provides methods for building dynamic notarization transactions.
#[wasm_bindgen(js_class = NotarizationBuilderDynamic)]
impl WasmNotarizationBuilderDynamic {
    /// Adds a state to the notarization using binary data.
    ///
    /// # Arguments
    /// * `data` - Binary data representing the state.
    /// * `metadata` - Optional metadata associated with the state.
    #[wasm_bindgen(js_name = withBytesState)]
    pub fn with_bytes_state(self, data: Uint8Array, metadata: Option<String>) -> Self {
        self.0.with_bytes_state(data.to_vec(), metadata).into()
    }

    /// Adds a state to the notarization using a string.
    ///
    /// # Arguments
    /// * `data` - String data representing the state.
    /// * `metadata` - Optional metadata associated with the state.
    #[wasm_bindgen(js_name = withStringState)]
    pub fn with_string_state(self, data: String, metadata: Option<String>) -> Self {
        self.0.with_string_state(data, metadata).into()
    }

    /// Adds an immutable description to the notarization.
    ///
    /// # Arguments
    /// * `description` - A string describing the notarization.
    #[wasm_bindgen(js_name = withImmutableDescription)]
    pub fn with_immutable_description(self, description: String) -> Self {
        self.0.with_immutable_description(description).into()
    }

    /// Adds updatable metadata to the notarization.
    ///
    /// # Arguments
    /// * `metadata` - A string representing the metadata.
    #[wasm_bindgen(js_name = withUpdatableMetadata)]
    pub fn with_updatable_metadata(self, metadata: String) -> Self {
        self.0.with_updatable_metadata(metadata).into()
    }

    /// Creates a new dynamic notarization builder.
    #[wasm_bindgen()]
    pub fn dynamic() -> Self {
        NotarizationBuilder::<Dynamic>::dynamic().into()
    }

    /// Adds a transfer lock to the notarization.
    ///
    /// # Arguments
    /// * `lock` - A `TimeLock` specifying the transfer lock.
    #[wasm_bindgen(js_name = withTransferLock)]
    pub fn with_transfer_lock(self, lock: WasmTimeLock) -> Self {
        self.0.with_transfer_lock(lock.0).into()
    }

    /// Finalizes the notarization builder and returns a transaction builder
    /// that can be used to build and execute the final transaction on the ledger.
    ///
    /// # Returns
    /// A `TransactionBuilder` to build and execute the transaction.
    #[wasm_bindgen()]
    pub fn finish(self) -> Result<WasmTransactionBuilder> {
        let js_value: JsValue = WasmCreateNotarizationDynamic::new(self).into();
        Ok(WasmTransactionBuilder::new(js_value.unchecked_into()))
    }
}
