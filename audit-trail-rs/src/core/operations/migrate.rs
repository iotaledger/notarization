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
    pub(crate) async fn migrate<C>(
        client: &C,
        trail_id: ObjectID,
        cap_id: ObjectID,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_trail_transaction(client, trail_id, cap_id, "migrate", |ptb| {
            let clock = move_utils::get_clock_ref(ptb);
            Ok(vec![clock])
        })
        .await
    }
}
