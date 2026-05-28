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

/// Tag-registry API scoped to a specific trail.
///
/// @remarks
/// The registry defines the canonical set of tags that record writes and {@link RoleTags}
/// restrictions may reference.
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
    /// Lists every tag in the trail's registry alongside its current usage count.
    ///
    /// @returns Tag entries sorted alphabetically by tag name.
    ///
    /// @throws When the trail object cannot be fetched.
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

    /// Builds a transaction that adds a tag to the trail registry.
    ///
    /// @remarks
    /// Inserted with a usage count of zero. The on-chain call aborts when the tag is already in
    /// the registry. Added tags become available to future tagged record writes and role-tag
    /// restrictions.
    ///
    /// Requires the {@link Permission.AddRecordTags} permission.
    ///
    /// @param tag - Tag name to add to the registry.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link AddRecordTag} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<AddRecordTag>")]
    pub fn add(&self, tag: String) -> Result<WasmTransactionBuilder> {
        let tx = self.require_write()?.trail(self.trail_id).tags().add(tag).into_inner();
        Ok(into_transaction_builder(WasmAddRecordTag(tx)))
    }

    /// Builds a transaction that removes a tag from the trail registry.
    ///
    /// @remarks
    /// The tag must currently be in the registry and must not be referenced by any record or
    /// role-tag restriction; the on-chain call aborts otherwise.
    ///
    /// Requires the {@link Permission.DeleteRecordTags} permission.
    ///
    /// @param tag - Tag name to remove from the registry.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link RemoveRecordTag} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
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
