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

use crate::trail::{
    WasmUpdateDeleteRecordWindow, WasmUpdateDeleteTrailLock, WasmUpdateLockingConfig, WasmUpdateWriteLock,
};
use crate::types::{WasmLockingConfig, WasmLockingWindow, WasmTimeLock};

/// Locking API scoped to a specific trail.
///
/// @remarks
/// Updates the trail's {@link LockingConfig} and queries whether an individual record is currently
/// locked against deletion.
#[derive(Clone)]
#[wasm_bindgen(js_name = TrailLocking, inspectable)]
pub struct WasmTrailLocking {
    pub(crate) read_only: AuditTrailClientReadOnly,
    pub(crate) full: Option<AuditTrailClient<WasmTransactionSigner>>,
    pub(crate) trail_id: ObjectID,
}

impl WasmTrailLocking {
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
    /// Builds a transaction that replaces the full locking configuration.
    ///
    /// @remarks
    /// Overwrites all three locking dimensions at once: delete-record window, delete-trail lock,
    /// and write lock. `config.deleteTrailLock` must not be {@link TimeLock.withUntilDestroyed};
    /// the on-chain call aborts otherwise.
    ///
    /// Requires the {@link Permission.UpdateLockingConfig} permission.
    ///
    /// @param config - Replacement {@link LockingConfig}.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link UpdateLockingConfig} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
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

    /// Builds a transaction that updates only the delete-record window.
    ///
    /// @remarks
    /// Replaces the trail's `deleteRecordWindow`. Records currently inside the new window
    /// immediately become locked against deletion.
    ///
    /// Requires the {@link Permission.UpdateLockingConfigForDeleteRecord} permission.
    ///
    /// @param window - Replacement {@link LockingWindow}.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link UpdateDeleteRecordWindow} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
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

    /// Builds a transaction that updates only the delete-trail lock.
    ///
    /// @remarks
    /// Replaces the trail's `deleteTrailLock`. The new lock must not be
    /// {@link TimeLock.withUntilDestroyed}; the on-chain call aborts otherwise.
    ///
    /// Requires the {@link Permission.UpdateLockingConfigForDeleteTrail} permission.
    ///
    /// @param lock - Replacement delete-trail {@link TimeLock}.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link UpdateDeleteTrailLock} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
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

    /// Builds a transaction that updates only the write lock.
    ///
    /// @remarks
    /// Replaces the trail's `writeLock`. While the new lock is active, {@link TrailRecords.add}
    /// aborts on-chain. {@link TimeLock.withUntilDestroyed} is permitted here.
    ///
    /// Requires the {@link Permission.UpdateLockingConfigForWrite} permission.
    ///
    /// @param lock - Replacement write {@link TimeLock}.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the {@link UpdateWriteLock} transaction.
    ///
    /// @throws When the wrapper was created from a read-only client.
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

    /// Returns whether a record is currently locked against deletion.
    ///
    /// @remarks
    /// Evaluates the trail's `deleteRecordWindow` against the record at `sequenceNumber` and the
    /// current clock time.
    ///
    /// @param sequenceNumber - Sequence number of the record to inspect.
    ///
    /// @returns `true` when the record is still inside the delete-record window, `false`
    /// otherwise.
    ///
    /// @throws When no record exists at `sequenceNumber`.
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
