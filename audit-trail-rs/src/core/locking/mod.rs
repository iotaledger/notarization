// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Locking configuration APIs for audit trails.

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
}

impl<'a, C> TrailLocking<'a, C> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self { client, trail_id }
    }

    /// Replaces the full locking configuration for the trail.
    ///
    /// This overwrites all three locking dimensions at once: record delete window, trail delete lock, and
    /// write lock.
    pub fn update<S>(&self, config: LockingConfig) -> TransactionBuilder<UpdateLockingConfig>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(UpdateLockingConfig::new(self.trail_id, owner, config))
    }

    /// Updates only the delete-record window.
    pub fn update_delete_record_window<S>(&self, window: LockingWindow) -> TransactionBuilder<UpdateDeleteRecordWindow>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(UpdateDeleteRecordWindow::new(self.trail_id, owner, window))
    }

    /// Updates only the delete-trail time lock.
    pub fn update_delete_trail_lock<S>(&self, lock: TimeLock) -> TransactionBuilder<UpdateDeleteTrailLock>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(UpdateDeleteTrailLock::new(self.trail_id, owner, lock))
    }

    /// Updates only the write lock.
    pub fn update_write_lock<S>(&self, lock: TimeLock) -> TransactionBuilder<UpdateWriteLock>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(UpdateWriteLock::new(self.trail_id, owner, lock))
    }

    /// Returns `true` when the given record is currently locked against deletion.
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
