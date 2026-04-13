// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use audit_trail::{AuditTrailClient, AuditTrailClientReadOnly};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result, WasmResult};
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::into_transaction_builder;
use wasm_bindgen::prelude::*;

use crate::trail::{WasmAddRecordTag, WasmRemoveRecordTag};
use crate::types::WasmRecordTagEntry;

#[derive(Clone)]
#[wasm_bindgen(js_name = TrailTags, inspectable)]
pub struct WasmTrailTags {
    pub(crate) read_only: AuditTrailClientReadOnly,
    pub(crate) full: Option<AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
}

impl WasmTrailTags {
    fn require_write(&self) -> Result<&AuditTrailClient<WasmTransactionSigner>> {
        self.full.as_ref().ok_or_else(|| {
            wasm_error(anyhow!(
                "TrailTags was created from a read-only client; this operation requires AuditTrailClient"
            ))
        })
    }
}

#[wasm_bindgen(js_class = TrailTags)]
impl WasmTrailTags {
    pub async fn list(&self) -> Result<Vec<WasmRecordTagEntry>> {
        let trail = self.read_only.trail(self.trail_id).get().await.wasm_result()?;
        let mut tags: Vec<WasmRecordTagEntry> = trail
            .tags
            .iter()
            .map(|(tag, usage_count)| (tag.clone(), *usage_count).into())
            .collect();
        tags.sort_unstable_by(|left, right| left.tag.cmp(&right.tag));
        Ok(tags)
    }

    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<AddRecordTag>")]
    pub fn add(&self, tag: String) -> Result<WasmTransactionBuilder> {
        let tx = self.require_write()?.trail(self.trail_id).tags().add(tag).into_inner();
        Ok(into_transaction_builder(WasmAddRecordTag(tx)))
    }

    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<RemoveRecordTag>")]
    pub fn remove(&self, tag: String) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .tags()
            .remove(tag)
            .into_inner();
        Ok(into_transaction_builder(WasmRemoveRecordTag(tx)))
    }
}
