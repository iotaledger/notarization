// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::{IotaTransactionBlockEffects, IotaTransactionBlockEvents};
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::operations::LockingOps;
use crate::core::types::{LockingConfig, LockingWindow};
use crate::error::Error;

#[derive(Debug, Clone)]
pub struct UpdateLockingConfig {
    trail_id: ObjectID,
    owner: IotaAddress,
    config: LockingConfig,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateLockingConfig {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, config: LockingConfig) -> Self {
        Self {
            trail_id,
            owner,
            config,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        LockingOps::update_locking_config(client, self.trail_id, self.owner, self.config.clone()).await
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

#[derive(Debug, Clone)]
pub struct UpdateDeleteRecordWindow {
    trail_id: ObjectID,
    owner: IotaAddress,
    window: LockingWindow,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl UpdateDeleteRecordWindow {
    pub fn new(trail_id: ObjectID, owner: IotaAddress, window: LockingWindow) -> Self {
        Self {
            trail_id,
            owner,
            window,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        LockingOps::update_delete_record_window(client, self.trail_id, self.owner, self.window.clone()).await
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
