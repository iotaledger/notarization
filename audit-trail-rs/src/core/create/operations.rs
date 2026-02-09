// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::ident_str;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::transaction::ProgrammableTransaction;

use crate::core::types::{Data, ImmutableMetadata, LockingConfig};
use crate::core::utils;
use crate::error::Error;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;

pub(super) struct CreateOps;

impl CreateOps {
    pub(super) fn create_trail(
        package_id: ObjectID,
        initial_data: Option<Data>,
        initial_record_metadata: Option<String>,
        locking_config: LockingConfig,
        trail_metadata: Option<ImmutableMetadata>,
        updatable_metadata: Option<String>,
    ) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let initial_data_arg = match initial_data {
            Some(data) => utils::ptb_pure(&mut ptb, "initial_data", Some(data))?,
            None => utils::ptb_pure::<Option<Data>>(&mut ptb, "initial_data", None)?,
        };

        let initial_record_metadata = utils::ptb_pure(&mut ptb, "initial_record_metadata", initial_record_metadata)?;
        let locking_config = utils::ptb_pure(&mut ptb, "locking_config", locking_config)?;

        let trail_metadata = match trail_metadata {
            Some(metadata) => metadata.to_ptb(&mut ptb, package_id)?,
            None => utils::ptb_pure::<Option<ImmutableMetadata>>(&mut ptb, "trail_metadata", None)?,
        };

        let updatable_metadata = utils::ptb_pure(&mut ptb, "updatable_metadata", updatable_metadata)?;
        let clock = utils::get_clock_ref(&mut ptb);

        ptb.programmable_move_call(
            package_id,
            ident_str!("main").into(),
            ident_str!("create").into(),
            vec![],
            vec![
                initial_data_arg,
                initial_record_metadata,
                locking_config,
                trail_metadata,
                updatable_metadata,
                clock,
            ],
        );

        Ok(ptb.finish())
    }
}
