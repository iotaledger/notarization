// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use iota_interaction::ident_str;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::{Argument, ProgrammableTransaction};

use crate::core::types::{ImmutableMetadata, InitialRecord, LockingConfig};
use crate::core::utils;
use crate::error::Error;

pub(super) struct CreateOps;

pub(super) struct CreateTrailArgs {
    pub audit_trail_package_id: ObjectID,
    pub tf_components_package_id: ObjectID,
    pub admin: IotaAddress,
    pub initial_record: Option<InitialRecord>,
    pub locking_config: LockingConfig,
    pub trail_metadata: Option<ImmutableMetadata>,
    pub updatable_metadata: Option<String>,
    pub record_tags: HashSet<String>,
}

impl CreateOps {
    pub(super) fn create_trail(args: CreateTrailArgs) -> Result<ProgrammableTransaction, Error> {
        let mut ptb = ProgrammableTransactionBuilder::new();
        let CreateTrailArgs {
            audit_trail_package_id,
            tf_components_package_id,
            admin,
            initial_record,
            locking_config,
            trail_metadata,
            updatable_metadata,
            record_tags,
        } = args;

        let initial_record = initial_record.ok_or_else(|| {
            Error::InvalidArgument(
                "initial_record is required to infer trail record type; use `with_initial_record(...)`".to_string(),
            )
        })?;
        let data_tag = initial_record.data.tag();
        let initial_record_tag = InitialRecord::tag(audit_trail_package_id, &data_tag);
        let initial_record_arg = initial_record.into_ptb(&mut ptb, audit_trail_package_id)?;
        let initial_record = utils::option_to_move(Some(initial_record_arg), initial_record_tag, &mut ptb)
            .map_err(|e| Error::InvalidArgument(format!("failed to build initial_record option: {e}")))?;
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

        let record_tags = {
            let mut record_tags = record_tags.into_iter().collect::<Vec<_>>();
            record_tags.sort();
            utils::ptb_pure(&mut ptb, "record_tags", record_tags)?
        };
        let clock = utils::get_clock_ref(&mut ptb);

        ptb.programmable_move_call(
            audit_trail_package_id,
            ident_str!("audit_trail").into(),
            ident_str!("create").into(),
            vec![data_tag],
            vec![
                initial_record,
                locking_config,
                trail_metadata,
                updatable_metadata,
                record_tags,
                clock,
            ],
        );

        Ok(ptb.finish())
    }
}
