// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::ident_str;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::{Argument, ProgrammableTransaction};

use crate::core::types::{Data, ImmutableMetadata, LockingConfig};
use crate::core::utils;
use crate::error::Error;

pub(super) struct CreateOps;

impl CreateOps {
    pub(super) fn create_trail(
        audit_trail_package_id: ObjectID,
        tf_components_package_id: ObjectID,
        admin: IotaAddress,
        initial_data: Option<Data>,
        initial_record_metadata: Option<String>,
        locking_config: LockingConfig,
        trail_metadata: Option<ImmutableMetadata>,
        updatable_metadata: Option<String>,
    ) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();

        let data_tag = Data::tag(audit_trail_package_id);
        let initial_data_arg = match initial_data {
            Some(data) => data.to_option_ptb(&mut ptb, audit_trail_package_id)?,
            None => utils::option_to_move(None, data_tag.clone(), &mut ptb)
                .map_err(|e| Error::InvalidArgument(format!("failed to build initial_data option: {e}")))?,
        };

        let initial_record_metadata = utils::ptb_pure(&mut ptb, "initial_record_metadata", initial_record_metadata)?;
        let locking_config = locking_config.to_ptb(&mut ptb, audit_trail_package_id, tf_components_package_id)?;

        let immutable_metadata_tag = ImmutableMetadata::tag(audit_trail_package_id);

        let trail_metadata = match trail_metadata {
            Some(metadata) => {
                let metadata_arg = metadata.to_ptb(&mut ptb, audit_trail_package_id)?;
                utils::option_to_move(Some(metadata_arg), immutable_metadata_tag, &mut ptb)
                    .map_err(|e| Error::InvalidArgument(format!("failed to build trail_metadata option: {e}")))?
            }
            None => utils::option_to_move(None, immutable_metadata_tag, &mut ptb)
                .map_err(|e| Error::InvalidArgument(format!("failed to build trail_metadata option: {e}")))?,
        };

        let updatable_metadata = utils::ptb_pure(&mut ptb, "updatable_metadata", updatable_metadata)?;
        let clock = utils::get_clock_ref(&mut ptb);

        let result = ptb.programmable_move_call(
            audit_trail_package_id,
            ident_str!("main").into(),
            ident_str!("create").into(),
            vec![data_tag],
            vec![
                initial_data_arg,
                initial_record_metadata,
                locking_config,
                trail_metadata,
                updatable_metadata,
                clock,
            ],
        );

        let cap = match result {
            Argument::Result(idx) => Argument::NestedResult(idx, 0),
            _ => unreachable!("programmable_move_call should always return Argument::Result"),
        };
        ptb.transfer_arg(admin, cap);

        Ok(ptb.finish())
    }
}
