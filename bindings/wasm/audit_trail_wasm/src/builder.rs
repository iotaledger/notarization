// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trail::core::builder::AuditTrailBuilder;
use iota_interaction_ts::wasm_error::Result;
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::{into_transaction_builder, parse_wasm_iota_address};
use product_common::bindings::WasmIotaAddress;
use wasm_bindgen::prelude::*;

use crate::trail::WasmCreateTrail;
use crate::types::WasmLockingConfig;

/// Trail-creation builder exposed to wasm consumers.
#[wasm_bindgen(js_name = AuditTrailBuilder, inspectable)]
pub struct WasmAuditTrailBuilder(pub(crate) AuditTrailBuilder);

#[wasm_bindgen(js_class = AuditTrailBuilder)]
impl WasmAuditTrailBuilder {
    /// Sets the initial record using a UTF-8 string payload.
    #[wasm_bindgen(js_name = withInitialRecordString)]
    pub fn with_initial_record_string(self, data: String, metadata: Option<String>, tag: Option<String>) -> Self {
        Self(self.0.with_initial_record_parts(data, metadata, tag))
    }

    /// Sets the initial record using raw bytes.
    #[wasm_bindgen(js_name = withInitialRecordBytes)]
    pub fn with_initial_record_bytes(
        self,
        data: js_sys::Uint8Array,
        metadata: Option<String>,
        tag: Option<String>,
    ) -> Self {
        Self(self.0.with_initial_record_parts(data.to_vec(), metadata, tag))
    }

    /// Sets immutable metadata for the trail.
    #[wasm_bindgen(js_name = withTrailMetadata)]
    pub fn with_trail_metadata(self, name: String, description: Option<String>) -> Self {
        Self(self.0.with_trail_metadata_parts(name, description))
    }

    /// Sets mutable metadata for the trail.
    #[wasm_bindgen(js_name = withUpdatableMetadata)]
    pub fn with_updatable_metadata(self, metadata: String) -> Self {
        Self(self.0.with_updatable_metadata(metadata))
    }

    /// Sets the locking configuration for the trail.
    #[wasm_bindgen(js_name = withLockingConfig)]
    pub fn with_locking_config(self, config: WasmLockingConfig) -> Self {
        Self(self.0.with_locking_config(config.into()))
    }

    /// Sets the canonical list of record tags owned by the trail.
    #[wasm_bindgen(js_name = withRecordTags)]
    pub fn with_record_tags(self, tags: Vec<String>) -> Self {
        Self(self.0.with_record_tags(tags))
    }

    /// Sets the initial admin address.
    #[wasm_bindgen(js_name = withAdmin)]
    pub fn with_admin(self, admin: WasmIotaAddress) -> Result<Self> {
        let admin = parse_wasm_iota_address(&admin)?;
        Ok(Self(self.0.with_admin(admin)))
    }

    /// Finalizes the builder into a transaction wrapper.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<CreateTrail>")]
    pub fn finish(self) -> Result<WasmTransactionBuilder> {
        Ok(into_transaction_builder(WasmCreateTrail::new(self)))
    }
}
