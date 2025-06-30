// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use js_sys::Uint8Array;
use notarization::core::types::{NotarizationMethod, Data, ImmutableMetadata, LockMetadata, State};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::wasm_time_lock::WasmTimeLock;

#[wasm_bindgen(js_name = Empty, inspectable)]
pub struct WasmEmpty;

impl From<()> for WasmEmpty {
    fn from(_: ()) -> WasmEmpty {
        WasmEmpty
    }
}

#[wasm_bindgen(js_name = Data, inspectable)]
pub struct WasmData(pub(crate) Data);

#[wasm_bindgen(js_class = Data)]
impl WasmData {
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> JsValue {
        match &self.0 {
            Data::Bytes(bytes) => JsValue::from(bytes.clone()),
            Data::Text(text) => JsValue::from(text),
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        match &self.0 {
            Data::Bytes(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            Data::Text(text) => text.to_string(),
        }
    }

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

#[wasm_bindgen(js_name = State, inspectable)]
pub struct WasmState(pub(crate) State);

#[wasm_bindgen(js_class = State)]
impl WasmState {
    #[wasm_bindgen(getter)]
    pub fn data(&self) -> WasmData {
        self.0.data.clone().into()
    }

    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> Option<String> {
        self.0.metadata.clone()
    }

    #[wasm_bindgen(js_name = fromString)]
    pub fn from_string(data: String, metadata: Option<String>) -> Self {
        WasmState(State::from_string(data, metadata))
    }

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

#[wasm_bindgen(js_name = LockMetadata, getter_with_clone, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmLockMetadata {
    #[wasm_bindgen(js_name = updateLock)]
    pub update_lock: WasmTimeLock,
    #[wasm_bindgen(js_name = deleteLock)]
    pub delete_lock: WasmTimeLock,
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

#[wasm_bindgen(js_name = ImmutableMetadata, inspectable)]
pub struct WasmImmutableMetadata(pub(crate) ImmutableMetadata);

#[wasm_bindgen(js_class = ImmutableMetadata)]
impl WasmImmutableMetadata {
    /// Timestamp when the `Notarization` was created
    #[wasm_bindgen(js_name = createdAt, getter)]
    pub fn created_at(&self) -> u64 {
        self.0.created_at
    }
    /// Description of the `Notarization`
    #[wasm_bindgen(getter)]
    pub fn description(&self) -> Option<String> {
        self.0.description.clone()
    }
    /// Optional lock metadata for `Notarization`
    #[wasm_bindgen(getter)]
    pub fn locking(&self) -> Option<WasmLockMetadata> {
        self.0.locking.clone().map(|l| l.into())
    }
}

#[wasm_bindgen(js_name = NotarizationMethod)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmNotarizationMethod {
    Dynamic = "Dynamic",
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
            WasmNotarizationMethod::__Invalid => panic!("The NotarizationMethod {:?} is not known", value),
        }
    }
}
