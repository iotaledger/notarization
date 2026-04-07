// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Internal helpers that build record-tag registry transactions.
//!
//! These helpers encode updates to the trail-owned tag registry and select the corresponding tag-management
//! permissions.

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use crate::core::internal::tx;
use crate::core::types::Permission;
use crate::error::Error;

/// Internal namespace for tag-registry transaction construction.
pub(super) struct TagsOps;

impl TagsOps {
    /// Builds the `add_record_tag` call.
    pub(super) async fn add_record_tag<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        tag: String,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::AddRecordTags,
            "add_record_tag",
            |ptb, _| {
                let tag_arg = tx::ptb_pure(ptb, "tag", tag)?;
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![tag_arg, clock])
            },
        )
        .await
    }

    /// Builds the `remove_record_tag` call.
    pub(super) async fn remove_record_tag<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        tag: String,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteRecordTags,
            "remove_record_tag",
            |ptb, _| {
                let tag_arg = tx::ptb_pure(ptb, "tag", tag)?;
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![tag_arg, clock])
            },
        )
        .await
    }
}
