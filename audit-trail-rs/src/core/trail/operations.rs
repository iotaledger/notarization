// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::OptionalSync;
use product_common::core_client::CoreClientReadOnly;

use crate::core::operations;
use crate::core::utils;
use crate::error::Error;

pub(super) struct TrailOps;

impl TrailOps {
    pub(super) async fn update_metadata<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        metadata: Option<String>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction_for_owner(client, trail_id, owner, "update_metadata", |ptb, _| {
            let metadata_arg = utils::ptb_pure(ptb, "new_metadata", metadata)?;
            let clock = utils::get_clock_ref(ptb);
            Ok(vec![metadata_arg, clock])
        })
        .await
    }
}
