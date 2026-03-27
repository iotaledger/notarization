// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use audit_trails::{AuditTrailClient, AuditTrailClientReadOnly};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmTransactionSigner;
use iota_interaction_ts::wasm_error::{wasm_error, Result, WasmResult};
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::into_transaction_builder;
use wasm_bindgen::prelude::*;

use crate::trail::{
    WasmUpdateDeleteRecordWindow, WasmUpdateDeleteTrailLock, WasmUpdateLockingConfig, WasmUpdateWriteLock,
};
use crate::types::{WasmLockingConfig, WasmLockingWindow, WasmTimeLock};

#[derive(Clone)]
#[wasm_bindgen(js_name = TrailLocking, inspectable)]
pub struct WasmTrailLocking {
    pub(crate) read_only: AuditTrailClientReadOnly,
    pub(crate) full: Option<AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
}

impl WasmTrailLocking {
    /// Returns the writable client for locking updates.
    ///
    /// Throws when this wrapper was created from `AuditTrailClientReadOnly`.
    fn require_write(&self) -> Result<&AuditTrailClient<WasmTransactionSigner>> {
        self.full.as_ref().ok_or_else(|| {
            wasm_error(anyhow!(
                "TrailLocking was created from a read-only client; this operation requires AuditTrailClient"
            ))
        })
    }
}

#[wasm_bindgen(js_class = TrailLocking)]
impl WasmTrailLocking {
    #[wasm_bindgen(unchecked_return_type = "TransactionBuilder<UpdateLockingConfig>")]
    pub fn update(&self, config: WasmLockingConfig) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .locking()
            .update(config.into())
            .into_inner();
        Ok(into_transaction_builder(WasmUpdateLockingConfig(tx)))
    }

    #[wasm_bindgen(js_name = updateDeleteRecordWindow, unchecked_return_type = "TransactionBuilder<UpdateDeleteRecordWindow>")]
    pub fn update_delete_record_window(&self, window: WasmLockingWindow) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .locking()
            .update_delete_record_window(window.into())
            .into_inner();
        Ok(into_transaction_builder(WasmUpdateDeleteRecordWindow(tx)))
    }

    #[wasm_bindgen(js_name = updateDeleteTrailLock, unchecked_return_type = "TransactionBuilder<UpdateDeleteTrailLock>")]
    pub fn update_delete_trail_lock(&self, lock: WasmTimeLock) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .locking()
            .update_delete_trail_lock(lock.into())
            .into_inner();
        Ok(into_transaction_builder(WasmUpdateDeleteTrailLock(tx)))
    }

    #[wasm_bindgen(js_name = updateWriteLock, unchecked_return_type = "TransactionBuilder<UpdateWriteLock>")]
    pub fn update_write_lock(&self, lock: WasmTimeLock) -> Result<WasmTransactionBuilder> {
        let tx = self
            .require_write()?
            .trail(self.trail_id)
            .locking()
            .update_write_lock(lock.into())
            .into_inner();
        Ok(into_transaction_builder(WasmUpdateWriteLock(tx)))
    }

    #[wasm_bindgen(js_name = isRecordLocked)]
    pub async fn is_record_locked(&self, sequence_number: u64) -> Result<bool> {
        self.read_only
            .trail(self.trail_id)
            .locking()
            .is_record_locked(sequence_number)
            .await
            .wasm_result()
    }
}
