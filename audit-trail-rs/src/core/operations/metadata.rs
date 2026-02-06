// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use super::AuditTrailImpl;
use crate::core::move_utils;
use crate::error::Error;

impl AuditTrailImpl {
    pub(crate) async fn update_metadata<C>(
        client: &C,
        trail_id: ObjectID,
        cap_id: ObjectID,
        new_metadata: Option<String>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_trail_transaction(client, trail_id, cap_id, "update_metadata", |ptb| {
            let meta = move_utils::ptb_pure(ptb, "new_metadata", new_metadata)?;
            let clock = move_utils::get_clock_ref(ptb);
            Ok(vec![meta, clock])
        })
        .await
    }
}
