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

pub(super) struct CreateTrailArgs {
    pub audit_trail_package_id: ObjectID,
    pub tf_components_package_id: ObjectID,
    pub admin: IotaAddress,
    pub initial_data: Option<Data>,
    pub initial_record_metadata: Option<String>,
    pub locking_config: LockingConfig,
    pub trail_metadata: Option<ImmutableMetadata>,
    pub updatable_metadata: Option<String>,
    pub available_record_tags: Vec<String>,
}

impl CreateOps {
    pub(super) fn create_trail(args: CreateTrailArgs) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();
        let CreateTrailArgs {
            audit_trail_package_id,
            tf_components_package_id,
            admin,
            initial_data,
            initial_record_metadata,
            locking_config,
            trail_metadata,
            updatable_metadata,
            mut available_record_tags,
        } = args;

        let initial_data = initial_data.ok_or_else(|| {
            Error::InvalidArgument(
                "initial_data is required to infer trail record type; use `with_initial_record(...)`".to_string(),
            )
        })?;
        let data_tag = initial_data.tag();
        let initial_data_arg = initial_data.into_option_ptb(&mut ptb, "initial_data")?;

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
        available_record_tags.sort();
        let available_record_tags = utils::ptb_pure(&mut ptb, "available_record_tags", available_record_tags)?;
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
                available_record_tags,
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
