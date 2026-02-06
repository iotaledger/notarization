// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use super::AuditTrailImpl;
use crate::core::move_utils;
use crate::core::types::LockingWindow;
use crate::error::Error;

impl AuditTrailImpl {
    pub(crate) async fn update_locking_config<C>(
        client: &C,
        trail_id: ObjectID,
        cap_id: ObjectID,
        new_config: crate::core::types::LockingConfig,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_trail_transaction(client, trail_id, cap_id, "update_locking_config", |ptb| {
            let config = move_utils::ptb_pure(ptb, "new_config", new_config)?;
            let clock = move_utils::get_clock_ref(ptb);
            Ok(vec![config, clock])
        })
        .await
    }

    pub(crate) async fn update_locking_config_for_delete_record<C>(
        client: &C,
        trail_id: ObjectID,
        cap_id: ObjectID,
        window: LockingWindow,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        Self::build_trail_transaction(
            client,
            trail_id,
            cap_id,
            "update_locking_config_for_delete_record",
            |ptb| {
                let window = move_utils::ptb_pure(ptb, "new_delete_record_lock", window)?;
                let clock = move_utils::get_clock_ref(ptb);
                Ok(vec![window, clock])
            },
        )
        .await
    }
}
