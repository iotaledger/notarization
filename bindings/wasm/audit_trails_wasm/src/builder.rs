// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trails::core::builder::AuditTrailBuilder;
use iota_interaction_ts::wasm_error::Result;
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::{into_transaction_builder, parse_wasm_iota_address};
use product_common::bindings::WasmIotaAddress;
use wasm_bindgen::prelude::*;

use crate::trail::WasmCreateTrail;
use crate::types::WasmLockingConfig;

#[wasm_bindgen(js_name = AuditTrailBuilder, inspectable)]
pub struct WasmAuditTrailBuilder(pub(crate) AuditTrailBuilder);

#[wasm_bindgen(js_class = AuditTrailBuilder)]
impl WasmAuditTrailBuilder {
    #[wasm_bindgen(js_name = withInitialRecordString)]
    pub fn with_initial_record_string(self, data: String, metadata: Option<String>) -> Self {
        Self(self.0.with_initial_record(data, metadata))
    }

    #[wasm_bindgen(js_name = withInitialRecordBytes)]
    pub fn with_initial_record_bytes(self, data: js_sys::Uint8Array, metadata: Option<String>) -> Self {
        Self(self.0.with_initial_record(data.to_vec(), metadata))
    }

    #[wasm_bindgen(js_name = withTrailMetadata)]
    pub fn with_trail_metadata(self, name: String, description: Option<String>) -> Self {
        Self(self.0.with_trail_metadata_parts(name, description))
    }

    #[wasm_bindgen(js_name = withUpdatableMetadata)]
    pub fn with_updatable_metadata(self, metadata: String) -> Self {
        Self(self.0.with_updatable_metadata(metadata))
    }

    #[wasm_bindgen(js_name = withLockingConfig)]
    pub fn with_locking_config(self, config: WasmLockingConfig) -> Self {
        Self(self.0.with_locking_config(config.into()))
    }

    #[wasm_bindgen(js_name = withAdmin)]
    pub fn with_admin(self, admin: WasmIotaAddress) -> Result<Self> {
        let admin = parse_wasm_iota_address(&admin)?;
        Ok(Self(self.0.with_admin(admin)))
    }

    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<CreateTrail>")]
    pub fn finish(self) -> Result<WasmTransactionBuilder> {
        Ok(into_transaction_builder(WasmCreateTrail::new(self)))
    }
}
