// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use crate::core::internal::tx;
use crate::core::types::Permission;
use crate::error::Error;

pub(super) struct TagsOps;

impl TagsOps {
    pub(super) async fn add_record_tag<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        tag: String,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::AddRecordTags,
            selected_capability_id,
            "add_record_tag",
            |ptb, _| {
                let tag_arg = tx::ptb_pure(ptb, "tag", tag)?;
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![tag_arg, clock])
            },
        )
        .await
    }

    pub(super) async fn remove_record_tag<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        tag: String,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteRecordTags,
            selected_capability_id,
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
