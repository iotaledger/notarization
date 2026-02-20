// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::{LockingConfig, LockingWindow, Permission};
use crate::core::{operations, utils};
use crate::error::Error;

pub(super) struct LockingOps;

impl LockingOps {
    pub(super) async fn update_locking_config<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        new_config: LockingConfig,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::UpdateLockingConfig,
            "update_locking_config",
            |ptb, _| {
                let config = new_config.to_ptb(ptb, client.package_id())?;
                let clock = utils::get_clock_ref(ptb);

                Ok(vec![config, clock])
            },
        )
        .await
    }

    pub(super) async fn update_delete_record_window<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        new_window: LockingWindow,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::UpdateLockingConfigForDeleteRecord,
            "update_locking_config_for_delete_record",
            |ptb, _| {
                let window = new_window.to_ptb(ptb, client.package_id())?;
                let clock = utils::get_clock_ref(ptb);

                Ok(vec![window, clock])
            },
        )
        .await
    }

    pub(super) async fn is_record_locked<C>(
        client: &C,
        trail_id: ObjectID,
        sequence_number: u64,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_read_only_transaction(client, trail_id, "is_record_locked", |ptb| {
            let sequence_number = utils::ptb_pure(ptb, "sequence_number", sequence_number)?;
            let clock = utils::get_clock_ref(ptb);

            Ok(vec![sequence_number, clock])
        })
        .await
    }
}
