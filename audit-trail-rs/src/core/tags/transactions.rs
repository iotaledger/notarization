// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Transaction payloads for tag-registry updates.

use async_trait::async_trait;
use iota_interaction::OptionalSync;
use iota_interaction::rpc_types::IotaTransactionBlockEffects;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use tokio::sync::OnceCell;

use super::operations::TagsOps;
use crate::error::Error;

/// Transaction that adds a record tag to the trail registry.
///
/// This extends the canonical tag registry owned by the trail.
#[derive(Debug, Clone)]
pub struct AddRecordTag {
    trail_id: ObjectID,
    owner: IotaAddress,
    tag: String,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl AddRecordTag {
    /// Creates an `AddRecordTag` transaction builder payload.
    pub fn new(trail_id: ObjectID, owner: IotaAddress, tag: String) -> Self {
        Self {
            trail_id,
            owner,
            tag,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        TagsOps::add_record_tag(client, self.trail_id, self.owner, self.tag.clone()).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for AddRecordTag {
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

/// Transaction that removes a record tag from the trail registry.
///
/// Removal only succeeds when the tag is no longer used by records or role-tag restrictions.
#[derive(Debug, Clone)]
pub struct RemoveRecordTag {
    trail_id: ObjectID,
    owner: IotaAddress,
    tag: String,
    cached_ptb: OnceCell<ProgrammableTransaction>,
}

impl RemoveRecordTag {
    /// Creates a `RemoveRecordTag` transaction builder payload.
    pub fn new(trail_id: ObjectID, owner: IotaAddress, tag: String) -> Self {
        Self {
            trail_id,
            owner,
            tag,
            cached_ptb: OnceCell::new(),
        }
    }

    async fn make_ptb<C>(&self, client: &C) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        TagsOps::remove_record_tag(client, self.trail_id, self.owner, self.tag.clone()).await
    }
}

#[cfg_attr(not(feature = "send-sync"), async_trait(?Send))]
#[cfg_attr(feature = "send-sync", async_trait)]
impl Transaction for RemoveRecordTag {
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
