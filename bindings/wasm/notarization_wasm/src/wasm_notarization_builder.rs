// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction_ts::wasm_error::Result;
use js_sys::Uint8Array;
use notarization::core::builder::{Dynamic, Locked, NotarizationBuilder};
use product_common::bindings::transaction::WasmTransactionBuilder;
use wasm_bindgen::prelude::*;

use crate::wasm_notarization::{WasmCreateNotarizationDynamic, WasmCreateNotarizationLocked};
use crate::wasm_time_lock::WasmTimeLock;

/// Builder for a "create Locked-Notarization" transaction.
///
/// @remarks
/// A Locked-Notarization is immutable after creation: its state and
/// updatable metadata are fixed for the lifetime of the object. Use this
/// builder to configure the initial state, immutable description, updatable
/// metadata, and optional `deleteLock`, then call
/// {@link NotarizationBuilderLocked.finish} to obtain a transaction builder.
///
/// On execution the transaction transfers the new notarization object to
/// the sender.
///
/// Emits a `LockedNotarizationCreated` event on success.
#[wasm_bindgen(js_name = NotarizationBuilderLocked, inspectable)]
pub struct WasmNotarizationBuilderLocked(pub(crate) NotarizationBuilder<Locked>);

impl From<NotarizationBuilder<Locked>> for WasmNotarizationBuilderLocked {
    fn from(val: NotarizationBuilder<Locked>) -> Self {
        WasmNotarizationBuilderLocked(val)
    }
}

#[wasm_bindgen(js_class = NotarizationBuilderLocked)]
impl WasmNotarizationBuilderLocked {
    /// Sets the initial state from a binary payload.
    ///
    /// @param data - The bytes to notarize.
    /// @param metadata - Optional metadata associated with this initial state.
    ///
    /// @returns The same builder, with the initial state configured.
    #[wasm_bindgen(js_name = withBytesState)]
    pub fn with_bytes_state(self, data: Uint8Array, metadata: Option<String>) -> Self {
        self.0.with_bytes_state(data.to_vec(), metadata).into()
    }

    /// Sets the initial state from a text payload.
    ///
    /// @param data - The string to notarize.
    /// @param metadata - Optional metadata associated with this initial state.
    ///
    /// @returns The same builder, with the initial state configured.
    #[wasm_bindgen(js_name = withStringState)]
    pub fn with_string_state(self, data: String, metadata: Option<String>) -> Self {
        self.0.with_string_state(data, metadata).into()
    }

    /// Sets the immutable description.
    ///
    /// @param description - Human-readable description fixed at creation. Pass
    /// `null` or `undefined` to leave the description unset.
    ///
    /// @returns The same builder, with the description configured.
    #[wasm_bindgen(js_name = withImmutableDescription)]
    pub fn with_immutable_description(self, description: Option<String>) -> Self {
        match description {
            Some(desc) => self.0.with_immutable_description(desc).into(),
            None => self,
        }
    }

    /// Sets the updatable metadata.
    ///
    /// @remarks
    /// On a Locked-Notarization the updatable metadata is fixed at creation
    /// just like the state — there is no client method that can change it
    /// afterwards.
    ///
    /// @param metadata - Updatable metadata string. Pass `null` or
    /// `undefined` to leave it unset.
    ///
    /// @returns The same builder, with the updatable metadata configured.
    #[wasm_bindgen(js_name = withUpdatableMetadata)]
    pub fn with_updatable_metadata(self, metadata: Option<String>) -> Self {
        match metadata {
            Some(meta) => self.0.with_updatable_metadata(meta).into(),
            None => self,
        }
    }

    /// Returns a fresh, unconfigured Locked-Notarization builder.
    ///
    /// @returns An empty {@link NotarizationBuilderLocked}.
    #[wasm_bindgen()]
    pub fn locked() -> Self {
        NotarizationBuilder::<Locked>::locked().into()
    }

    /// Sets the delete lock for the notarization.
    ///
    /// @remarks
    /// `deleteLock` cannot be {@link TimeLockType.UntilDestroyed} — submitting
    /// such a configuration aborts on-chain.
    ///
    /// @param lock - The {@link TimeLock} controlling when destruction is
    /// permitted.
    ///
    /// @returns The same builder, with the delete lock configured.
    #[wasm_bindgen(js_name = withDeleteLock)]
    pub fn with_delete_lock(self, lock: WasmTimeLock) -> Self {
        self.0.with_delete_lock(lock.0).into()
    }

    /// Finalizes the configuration and produces the transaction builder.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link CreateNotarizationLocked} transaction.
    ///
    /// @throws When the configured state, metadata, or lock combination is
    /// invalid for a Locked-Notarization.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<CreateNotarizationLocked>")]
    pub fn finish(self) -> Result<WasmTransactionBuilder> {
        let js_value: JsValue = WasmCreateNotarizationLocked::new(self).into();
        Ok(WasmTransactionBuilder::new(js_value.unchecked_into()))
    }
}

/// Builder for a "create Dynamic-Notarization" transaction.
///
/// @remarks
/// A Dynamic-Notarization can be updated after creation: state and updatable
/// metadata can be replaced via {@link NotarizationClient.updateState} and
/// {@link NotarizationClient.updateMetadata}, and ownership can be
/// transferred via {@link NotarizationClient.transferNotarization} when the
/// configured `transferLock` permits it.
///
/// On execution the transaction transfers the new notarization object to
/// the sender.
///
/// Emits a `DynamicNotarizationCreated` event on success.
#[wasm_bindgen(js_name = NotarizationBuilderDynamic)]
pub struct WasmNotarizationBuilderDynamic(pub(crate) NotarizationBuilder<Dynamic>);

impl From<NotarizationBuilder<Dynamic>> for WasmNotarizationBuilderDynamic {
    fn from(val: NotarizationBuilder<Dynamic>) -> Self {
        WasmNotarizationBuilderDynamic(val)
    }
}

#[wasm_bindgen(js_class = NotarizationBuilderDynamic)]
impl WasmNotarizationBuilderDynamic {
    /// Sets the initial state from a binary payload.
    ///
    /// @param data - The bytes to notarize.
    /// @param metadata - Optional metadata associated with this initial state.
    ///
    /// @returns The same builder, with the initial state configured.
    #[wasm_bindgen(js_name = withBytesState)]
    pub fn with_bytes_state(self, data: Uint8Array, metadata: Option<String>) -> Self {
        self.0.with_bytes_state(data.to_vec(), metadata).into()
    }

    /// Sets the initial state from a text payload.
    ///
    /// @param data - The string to notarize.
    /// @param metadata - Optional metadata associated with this initial state.
    ///
    /// @returns The same builder, with the initial state configured.
    #[wasm_bindgen(js_name = withStringState)]
    pub fn with_string_state(self, data: String, metadata: Option<String>) -> Self {
        self.0.with_string_state(data, metadata).into()
    }

    /// Sets the immutable description.
    ///
    /// @param description - Human-readable description fixed at creation. Pass
    /// `null` or `undefined` to leave the description unset.
    ///
    /// @returns The same builder, with the description configured.
    #[wasm_bindgen(js_name = withImmutableDescription)]
    pub fn with_immutable_description(self, description: Option<String>) -> Self {
        match description {
            Some(desc) => self.0.with_immutable_description(desc).into(),
            None => self,
        }
    }

    /// Sets the initial updatable metadata.
    ///
    /// @param metadata - Updatable metadata string. Pass `null` or
    /// `undefined` to leave it unset; it can still be updated later via
    /// {@link NotarizationClient.updateMetadata}.
    ///
    /// @returns The same builder, with the updatable metadata configured.
    #[wasm_bindgen(js_name = withUpdatableMetadata)]
    pub fn with_updatable_metadata(self, metadata: Option<String>) -> Self {
        match metadata {
            Some(meta) => self.0.with_updatable_metadata(meta).into(),
            None => self,
        }
    }

    /// Returns a fresh, unconfigured Dynamic-Notarization builder.
    ///
    /// @returns An empty {@link NotarizationBuilderDynamic}.
    #[wasm_bindgen()]
    pub fn dynamic() -> Self {
        NotarizationBuilder::<Dynamic>::dynamic().into()
    }

    /// Sets the transfer lock for the notarization.
    ///
    /// @remarks
    /// While the transfer lock is active,
    /// {@link NotarizationClient.transferNotarization} aborts on-chain. When
    /// the lock is {@link TimeLockType.None}, the resulting notarization
    /// carries no {@link LockMetadata} and is freely transferable.
    ///
    /// @param lock - The {@link TimeLock} controlling when ownership can be
    /// transferred.
    ///
    /// @returns The same builder, with the transfer lock configured.
    #[wasm_bindgen(js_name = withTransferLock)]
    pub fn with_transfer_lock(self, lock: WasmTimeLock) -> Self {
        self.0.with_transfer_lock(lock.0).into()
    }

    /// Finalizes the configuration and produces the transaction builder.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link CreateNotarizationDynamic} transaction.
    ///
    /// @throws When the configured state, metadata, or lock combination is
    /// invalid for a Dynamic-Notarization.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<CreateNotarizationDynamic>")]
    pub fn finish(self) -> Result<WasmTransactionBuilder> {
        let js_value: JsValue = WasmCreateNotarizationDynamic::new(self).into();
        Ok(WasmTransactionBuilder::new(js_value.unchecked_into()))
    }
}
