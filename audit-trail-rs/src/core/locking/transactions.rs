// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Transaction payloads for locking updates.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::operations::LockingOps;
use crate::core::types::{LockingConfig, LockingWindow, TimeLock};
use crate::error::Error;

/// Transaction that replaces the full locking configuration.
///
/// This writes the full `LockingConfig` object and therefore updates all locking dimensions in one call.
#[derive(Debug, Clone)]
pub struct UpdateLockingConfig {
    trail_id: ObjectID,
    owner: IotaAddress,
    config: LockingConfig,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateLockingConfig {
    /// Creates an `UpdateLockingConfig` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        config: LockingConfig,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            config,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        LockingOps::update_locking_config(
            client,
            self.trail_id,
            self.owner,
            self.config.clone(),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for UpdateLockingConfig {
    type Error = Error;
    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply<C>(self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}

/// Transaction that updates the delete-record window.
///
/// This updates only the rule that governs when individual records may be deleted.
#[derive(Debug, Clone)]
pub struct UpdateDeleteRecordWindow {
    trail_id: ObjectID,
    owner: IotaAddress,
    window: LockingWindow,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateDeleteRecordWindow {
    /// Creates an `UpdateDeleteRecordWindow` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        window: LockingWindow,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            window,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        LockingOps::update_delete_record_window(
            client,
            self.trail_id,
            self.owner,
            self.window.clone(),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for UpdateDeleteRecordWindow {
    type Error = Error;
    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply<C>(self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}

/// Transaction that updates the delete-trail lock.
///
/// This updates only the time lock guarding deletion of the entire trail object.
#[derive(Debug, Clone)]
pub struct UpdateDeleteTrailLock {
    trail_id: ObjectID,
    owner: IotaAddress,
    lock: TimeLock,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateDeleteTrailLock {
    /// Creates an `UpdateDeleteTrailLock` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        lock: TimeLock,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            lock,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        LockingOps::update_delete_trail_lock(
            client,
            self.trail_id,
            self.owner,
            self.lock.clone(),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for UpdateDeleteTrailLock {
    type Error = Error;
    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply<C>(self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}

/// Transaction that updates the write lock.
///
/// This updates only the time lock guarding future record writes.
#[derive(Debug, Clone)]
pub struct UpdateWriteLock {
    trail_id: ObjectID,
    owner: IotaAddress,
    lock: TimeLock,
    selected_capability_id: Option<ObjectID>,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateWriteLock {
    /// Creates an `UpdateWriteLock` transaction builder payload.
    pub fn new(
        trail_id: ObjectID,
        owner: IotaAddress,
        lock: TimeLock,
        selected_capability_id: Option<ObjectID>,
    ) -> Self {
        Self {
            trail_id,
            owner,
            lock,
            selected_capability_id,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        LockingOps::update_write_lock(
            client,
            self.trail_id,
            self.owner,
            self.lock.clone(),
            self.selected_capability_id,
        )
        .await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for UpdateWriteLock {
    type Error = Error;
    type Output = ();

    async fn build_programmable_transaction<C>(&self, client: &C) -> Result<ProgrammableTransaction, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        self.cached_ptb.get_or_try_init(|| self.make_ptb(client)).await.cloned()
    }

    async fn apply<C>(self, _: &mut IotaTransactionBlockEffects, _: &C) -> Result<Self::Output, Self::Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Ok(())
    }
}
