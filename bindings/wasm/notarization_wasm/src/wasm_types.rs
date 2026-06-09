// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use js_sys::Uint8Array;
use notarization::core::types::{Data, ImmutableMetadata, LockMetadata, NotarizationMethod, State};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::wasm_time_lock::WasmTimeLock;

/// An empty placeholder value returned by transaction-apply methods that have
/// no observable result.
#[wasm_bindgen(js_name = Empty, inspectable)]
pub struct WasmEmpty;

impl From<()> for WasmEmpty {
    fn from(_: ()) -> WasmEmpty {
        WasmEmpty
    }
}

/// A typed payload that can be notarized — either binary or text.
///
/// @remarks
/// `Data` is the inner payload of a {@link State}. Inspect its kind via
/// {@link Data.valueType} and pull the value out as bytes or text via
/// {@link Data.toBytes} or {@link Data.toString}.
#[wasm_bindgen(js_name = Data, inspectable)]
pub struct WasmData(pub(crate) Data);

#[wasm_bindgen(js_class = Data)]
impl WasmData {
    /// The raw value as either a `Uint8Array` (for binary payloads) or a
    /// `string` (for text payloads).
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> JsValue {
        match &self.0 {
            Data::Bytes(bytes) => JsValue::from(bytes.clone()),
            Data::Text(text) => JsValue::from(text),
        }
    }

    /// The runtime type of {@link Data.value} as a string: `"Uint8Array"` for
    /// binary payloads, `"String"` for text payloads.
    #[wasm_bindgen(getter, js_name = valueType)]
    pub fn value_type(&self) -> String {
        match &self.0 {
            Data::Bytes(_) => "Uint8Array".to_string(),
            Data::Text(_) => "String".to_string(),
        }
    }

    /// The size of the payload in bytes.
    ///
    /// @remarks
    /// For binary payloads this is the length of the underlying `Uint8Array`;
    /// for text payloads this is the UTF-8 byte length of the string.
    #[wasm_bindgen(getter, js_name = valueByteSize)]
    pub fn value_byte_size(&self) -> usize {
        match &self.0 {
            Data::Bytes(bytes) => bytes.len(),
            Data::Text(text) => text.len(),
        }
    }

    /// Returns the payload as a string.
    ///
    /// @remarks
    /// For binary payloads the bytes are interpreted as UTF-8; invalid byte
    /// sequences are replaced with the Unicode replacement character.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        match &self.0 {
            Data::Bytes(bytes) => String::from_utf8_lossy(bytes).to_string(),
            Data::Text(text) => text.to_string(),
        }
    }

    /// Returns the payload as a byte array.
    ///
    /// @remarks
    /// For text payloads the string is encoded as UTF-8.
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        match &self.0 {
            Data::Bytes(bytes) => bytes.clone(),
            Data::Text(text) => text.clone().as_bytes().to_vec(),
        }
    }
}

impl From<Data> for WasmData {
    fn from(value: Data) -> Self {
        WasmData(value)
    }
}

impl From<WasmData> for Data {
    fn from(value: WasmData) -> Self {
        serde_wasm_bindgen::from_value(value.into()).expect("From implementation works")
    }
}

/// The mutable content of a notarization — payload plus optional metadata.
///
/// @remarks
/// `State` pairs the notarized {@link Data} with an optional metadata string.
/// It is the primary content container of every notarization regardless of
/// the configured {@link NotarizationMethod}.
///
/// Mutability depends on the Notarization Method:
/// * `Dynamic`: `data` and `metadata` can be replaced after creation via {@link NotarizationClient.updateState}.
/// * `Locked`: the state is fixed at creation and cannot be replaced.
///
/// `data` and `metadata` can only be replaced together, in a single
/// {@link NotarizationClient.updateState} call. Every such replacement
/// increments the underlying notarization's `stateVersionCount` and updates
/// its `lastStateChangeAt` timestamp, even when only the `metadata` changes.
#[wasm_bindgen(js_name = State, inspectable)]
pub struct WasmState(pub(crate) State);

#[wasm_bindgen(js_class = State)]
impl WasmState {
    /// The notarized payload.
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> WasmData {
        self.0.data.clone().into()
    }

    /// The optional metadata associated with the current state version.
    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> Option<String> {
        self.0.metadata.clone()
    }

    /// Builds a state from a text payload.
    ///
    /// @param data - The string payload to notarize.
    /// @param metadata - Optional metadata associated with this state version.
    ///
    /// @returns A {@link State} carrying the given text payload.
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_string(data: String, metadata: Option<String>) -> Self {
        WasmState(State::from_string(data, metadata))
    }

    /// Builds a state from a binary payload.
    ///
    /// @param data - The bytes to notarize.
    /// @param metadata - Optional metadata associated with this state version.
    ///
    /// @returns A {@link State} carrying the given binary payload.
    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: Uint8Array, metadata: Option<String>) -> Self {
        WasmState(State::from_bytes(data.to_vec(), metadata))
    }
}

impl From<State> for WasmState {
    fn from(value: State) -> Self {
        WasmState(value)
    }
}

impl From<WasmState> for State {
    fn from(value: WasmState) -> Self {
        value.0
    }
}

/// Time-based access restrictions attached to a notarization at creation.
///
/// @remarks
/// `deleteLock` cannot be {@link TimeLockType.UntilDestroyed}, and its
/// unlock time must be no earlier than the unlock times of `updateLock` and
/// `transferLock` — on-chain creation aborts otherwise.
///
/// Permitted lock configurations depend on the {@link NotarizationMethod}:
/// * `Dynamic`: `updateLock` is fixed to {@link TimeLockType.None}; `transferLock` may carry any {@link TimeLock}
///   variant.
/// * `Locked`: both `updateLock` and `transferLock` are fixed to {@link TimeLockType.UntilDestroyed} —
///   Locked-Notarizations are non-transferable and their state is immutable.
#[wasm_bindgen(js_name = LockMetadata, getter_with_clone, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmLockMetadata {
    /// Lock gating state and metadata updates.
    ///
    /// Value depends on the Notarization Method:
    /// * `Dynamic`: fixed to {@link TimeLockType.None} — state and updatable metadata are always replaceable via:
    ///   * {@link NotarizationClient.updateState}
    ///   * {@link NotarizationClient.updateMetadata}
    /// * `Locked`: fixed to {@link TimeLockType.UntilDestroyed}.
    #[wasm_bindgen(js_name = updateLock)]
    pub update_lock: WasmTimeLock,
    /// Lock gating destruction. Cannot be {@link TimeLockType.UntilDestroyed};
    /// its unlock time must be ≥ both other locks' unlock times.
    #[wasm_bindgen(js_name = deleteLock)]
    pub delete_lock: WasmTimeLock,
    /// Lock gating ownership transfer.
    ///
    /// Value depends on the Notarization Method:
    /// * `Dynamic`: any {@link TimeLock} variant — controls when ownership transfer is permitted.
    /// * `Locked`: fixed to {@link TimeLockType.UntilDestroyed} — Locked-Notarizations are non-transferable.
    #[wasm_bindgen(js_name = transferLock)]
    pub transfer_lock: WasmTimeLock,
}

impl From<LockMetadata> for WasmLockMetadata {
    fn from(value: LockMetadata) -> Self {
        WasmLockMetadata {
            update_lock: WasmTimeLock(value.update_lock),
            delete_lock: WasmTimeLock(value.delete_lock),
            transfer_lock: WasmTimeLock(value.transfer_lock),
        }
    }
}

impl From<WasmLockMetadata> for LockMetadata {
    fn from(value: WasmLockMetadata) -> Self {
        serde_wasm_bindgen::from_value(value.into()).expect("From implementation works")
    }
}

/// The fixed-at-creation metadata of a notarization.
///
/// @remarks
/// Captures the values that cannot change after the notarization exists:
/// creation timestamp, optional human-readable description, and the optional
/// {@link LockMetadata}.
#[wasm_bindgen(js_name = ImmutableMetadata, inspectable)]
pub struct WasmImmutableMetadata(pub(crate) ImmutableMetadata);

#[wasm_bindgen(js_class = ImmutableMetadata)]
impl WasmImmutableMetadata {
    /// The creation timestamp, in milliseconds since the Unix epoch.
    #[wasm_bindgen(js_name = createdAt, getter)]
    pub fn created_at(&self) -> u64 {
        self.0.created_at
    }

    /// The optional human-readable description set at creation, if any.
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.0.description.clone()
    }

    /// The optional {@link LockMetadata} attached at creation.
    ///
    /// @remarks
    /// Presence depends on the Notarization Method:
    /// * `Dynamic`: absent when the Dynamic-Notarization carries no transfer lock; present otherwise.
    /// * `Locked`: always present.
    ///
    /// @returns The {@link LockMetadata}, or `null` when none is attached.
    #[wasm_bindgen(getter)]
    pub fn locking(&self) -> Option<WasmLockMetadata> {
        self.0.locking.clone().map(|l| l.into())
    }
}

/// Identifies the Notarization Method a notarization was created with.
///
/// @remarks
/// Returned by {@link OnChainNotarization.method} and
/// {@link NotarizationClientReadOnly.notarizationMethod}. The Notarization
/// Method is fixed at creation and determines which operations are permitted
/// on the notarization afterwards.
///
/// The set of Notarization Methods is closed in the current version of the
/// package but may be extended in future versions.
#[wasm_bindgen(js_name = NotarizationMethod)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmNotarizationMethod {
    /// Method whose state and updatable metadata can be updated after
    /// creation and which may optionally be transfer-locked.
    Dynamic = "Dynamic",
    /// Method whose state and updatable metadata are immutable after
    /// creation and whose destruction is gated by a `deleteLock`.
    Locked = "Locked",
}

impl From<NotarizationMethod> for WasmNotarizationMethod {
    fn from(value: NotarizationMethod) -> Self {
        match value {
            NotarizationMethod::Dynamic => WasmNotarizationMethod::Dynamic,
            NotarizationMethod::Locked => WasmNotarizationMethod::Locked,
        }
    }
}

impl From<WasmNotarizationMethod> for NotarizationMethod {
    fn from(value: WasmNotarizationMethod) -> Self {
        match value {
            WasmNotarizationMethod::Dynamic => NotarizationMethod::Dynamic,
            WasmNotarizationMethod::Locked => NotarizationMethod::Locked,
            WasmNotarizationMethod::__Invalid => panic!("The NotarizationMethod {value:?} is not known"),
        }
    }
}
