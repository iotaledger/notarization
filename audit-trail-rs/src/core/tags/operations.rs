// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::Permission;
use crate::core::{operations, utils};
use crate::error::Error;

pub(super) struct TagsOps;

impl TagsOps {
    pub(super) async fn add_record_tag<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        tag: String,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::AddRecordTags,
            "add_record_tag",
            |ptb, _| {
                let tag_arg = utils::ptb_pure(ptb, "tag", tag)?;
                let clock = utils::get_clock_ref(ptb);
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
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteRecordTags,
            "remove_record_tag",
            |ptb, _| {
                let tag_arg = utils::ptb_pure(ptb, "tag", tag)?;
                let clock = utils::get_clock_ref(ptb);
                Ok(vec![tag_arg, clock])
            },
        )
        .await
    }
}
