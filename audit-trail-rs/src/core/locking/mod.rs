// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Locking configuration APIs for Audit Trails.

use iota_interaction::types::base_types::ObjectID;
use iota_interaction::{IotaKeySignature, OptionalSync};
use product_common::core_client::CoreClient;
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;

use crate::core::trail::{AuditTrailFull, AuditTrailReadOnly};
use crate::core::types::{LockingConfig, LockingWindow, TimeLock};
use crate::error::Error;

mod operations;
mod transactions;

pub use transactions::{UpdateDeleteRecordWindow, UpdateDeleteTrailLock, UpdateLockingConfig, UpdateWriteLock};

use self::operations::LockingOps;

/// Locking API scoped to a specific trail.
///
/// This handle updates the trail's locking configuration and queries whether an individual record is currently
/// locked against deletion.
#[derive(Debug, Clone)]
pub struct TrailLocking<'a, C> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
    pub(crate) selected_capability_id: Option<ObjectID>,
}

impl<'a, C> TrailLocking<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID, selected_capability_id: Option<ObjectID>) -> Self {
        Self {
            client,
            trail_id,
            selected_capability_id,
        }
    }

    /// Uses the provided capability as the auth capability for subsequent write operations.
    pub fn using_capability(mut self, capability_id: ObjectID) -> Self {
        self.selected_capability_id = Some(capability_id);
        self
    }

    /// Replaces the full locking configuration for the trail.
    ///
    /// This overwrites all three locking dimensions at once: record delete window, trail delete lock, and
    /// write lock. The supplied [`LockingConfig`] is validated before the transaction is constructed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] when `config` contains:
    /// * `delete_record_window` using [`LockingWindow::CountBased`] with `count == 0` (mirrors the Move
    ///   `ECountWindowMustBePositive` abort).
    /// * `delete_trail_lock` using [`TimeLock::UntilDestroyed`] (mirrors the Move
    ///   `EUntilDestroyedNotSupportedForDeleteTrail` abort).
    pub fn update<S>(&self, config: LockingConfig) -> Result<TransactionBuilder<UpdateLockingConfig>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        config.validate()?;
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(UpdateLockingConfig::new(
            self.trail_id,
            owner,
            config,
            self.selected_capability_id,
        )))
    }

    /// Updates only the delete-record window.
    ///
    /// Count-based windows protect the last N records present in trail order at the start of each call that
    /// consults the window. `count` must be positive; pass [`LockingWindow::None`] to remove the lock.
    /// Large count values increase delete gas linearly because the on-chain check walks backward from the tail
    /// to determine the protected window's lower bound.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] when `window` is [`LockingWindow::CountBased`] with `count == 0`
    /// (mirrors the Move `ECountWindowMustBePositive` abort).
    pub fn update_delete_record_window<S>(
        &self,
        window: LockingWindow,
    ) -> Result<TransactionBuilder<UpdateDeleteRecordWindow>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        window.validate()?;
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(UpdateDeleteRecordWindow::new(
            self.trail_id,
            owner,
            window,
            self.selected_capability_id,
        )))
    }

    /// Updates only the delete-trail time lock.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArgument`] when `lock` is [`TimeLock::UntilDestroyed`]
    /// (mirrors the Move `EUntilDestroyedNotSupportedForDeleteTrail` abort).
    pub fn update_delete_trail_lock<S>(
        &self,
        lock: TimeLock,
    ) -> Result<TransactionBuilder<UpdateDeleteTrailLock>, Error>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        lock.validate_as_delete_trail_lock()?;
        let owner = self.client.sender_address();
        Ok(TransactionBuilder::new(UpdateDeleteTrailLock::new(
            self.trail_id,
            owner,
            lock,
            self.selected_capability_id,
        )))
    }

    /// Updates only the write lock.
    pub fn update_write_lock<S>(&self, lock: TimeLock) -> TransactionBuilder<UpdateWriteLock>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(UpdateWriteLock::new(
            self.trail_id,
            owner,
            lock,
            self.selected_capability_id,
        ))
    }

    /// Returns `true` when the given record is currently locked against deletion.
    ///
    /// For count-based windows, the check determines the protected window's lower bound by walking back
    /// from the current tail at call time; time-based locks are evaluated against the clock timestamp at
    /// call time. The result reflects the trail snapshot observed by this read-only call.
    ///
    /// # Errors
    ///
    /// Returns an error if the lock state cannot be computed from the current on-chain state.
    pub async fn is_record_locked(&self, sequence_number: u64) -> Result<bool, Error>
    where
        C: AuditTrailReadOnly,
    {
        let tx = LockingOps::is_record_locked(self.client, self.trail_id, sequence_number).await?;
        self.client.execute_read_only_transaction(tx).await
    }
}
