// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

use iota_interaction_ts::error::Result;
use product_common::bindings::transaction::WasmTransactionBuilder;

use notarization::core::builder::{Locked, Dynamic, NotarizationBuilder};

use crate::wasm_time_lock::WasmTimeLock;
use crate::wasm_notarization::WasmCreateNotarizationLocked;

#[wasm_bindgen(js_name = NotarizationBuilderLocked, inspectable)]
pub struct WasmNotarizationBuilderLocked(pub(crate) NotarizationBuilder<Locked>);

impl Into<WasmNotarizationBuilderLocked> for NotarizationBuilder<Locked> {
    fn into(self) -> WasmNotarizationBuilderLocked {
        WasmNotarizationBuilderLocked(self)
    }
}

#[wasm_bindgen(js_class = NotarizationBuilderLocked)]
impl WasmNotarizationBuilderLocked {
    
    #[wasm_bindgen(js_name = withBytesState)]
    pub fn with_bytes_state(self, data: Uint8Array, metadata: Option<String>) -> Self {self.0.with_bytes_state(data.to_vec(), metadata).into()}

    #[wasm_bindgen(js_name = withStringState)]
    pub fn with_string_state(self, data: String, metadata: Option<String>) -> Self {self.0.with_string_state(data, metadata).into()}

    #[wasm_bindgen(js_name = withImmutableDescription)]
    pub fn with_immutable_description(self, description: String) -> Self {self.0.with_immutable_description(description).into()}

    #[wasm_bindgen(js_name = withUpdateableMetadata)]
    pub fn with_updateable_metadata(self, metadata: String) -> Self {self.0.with_updateable_metadata(metadata).into()}

    #[wasm_bindgen()]
    pub fn locked() -> Self {NotarizationBuilder::<Locked>::locked().into()}

    #[wasm_bindgen(js_name = withDeleteAt)]
    pub fn with_delete_at(self, lock: WasmTimeLock) -> Self {self.0.with_delete_at(lock.0).into()}

    #[wasm_bindgen()]
    pub fn finish(self) -> Result<WasmTransactionBuilder> {
        let js_value: JsValue = WasmCreateNotarizationLocked::new(self).into();
        Ok(WasmTransactionBuilder::new(js_value.unchecked_into()))
    }
}

/*
#[wasm_bindgen(js_name = NotarizationBuilderDynamic)]
pub struct WasmNotarizationBuilderDynamic(pub(crate) NotarizationBuilder<Dynamic>);

#[wasm_bindgen(js_class = NotarizationBuilderDynamic)]
impl WasmNotarizationBuilderDynamic {
    pub fn dynamic() -> Self {}
    pub fn with_transfer_at(mut self, lock: TimeLock) -> Self {}
    pub fn finish(self) -> TransactionBuilder<CreateNotarization<Dynamic>> {}
}
*/