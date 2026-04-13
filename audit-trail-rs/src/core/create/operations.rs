// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Internal helpers that turn validated builder state into the trail-creation Move call.

use std::collections::HashSet;

use iota_interaction::ident_str;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::{Argument, ProgrammableTransaction};

use crate::core::internal::tx;
use crate::core::types::{Data, ImmutableMetadata, InitialRecord, LockingConfig};
use crate::error::Error;

/// Internal namespace for trail-creation transaction construction.
pub(super) struct CreateOps;

/// Normalized inputs required to build the `main::create` programmable transaction.
///
/// This keeps the public builder layer separate from the low-level PTB encoding logic.
pub(super) struct CreateTrailArgs {
    /// Audit-trail package used for generic type tags and Move calls.
    pub audit_trail_package_id: ObjectID,
    /// TfComponents package used by locking and capability-related values.
    pub tf_components_package_id: ObjectID,
    /// Address that should receive the initial admin capability.
    pub admin: IotaAddress,
    /// Optional first record inserted into the newly created trail.
    pub initial_record: Option<InitialRecord>,
    /// Initial locking rules for the trail.
    pub locking_config: LockingConfig,
    /// Immutable metadata stored at trail creation time.
    pub trail_metadata: Option<ImmutableMetadata>,
    /// Mutable metadata slot initialized together with the trail.
    pub updatable_metadata: Option<String>,
    /// Canonical set of record tags that may be used on the trail.
    pub record_tags: HashSet<String>,
}

impl CreateOps {
    /// Builds the programmable transaction that creates a new audit trail.
    ///
    /// Record tags are sorted before serialization so the resulting wire format is stable across
    /// equivalent `HashSet` inputs.
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

        let data_tag = Data::tag(audit_trail_package_id);
        let initial_record_tag = InitialRecord::tag(audit_trail_package_id);
        let initial_record = match initial_record {
            Some(initial_record) => {
                let initial_record_arg = initial_record.into_ptb(&mut ptb, audit_trail_package_id)?;
                tx::option_to_move(Some(initial_record_arg), initial_record_tag, &mut ptb)
            }
            None => tx::option_to_move(None, initial_record_tag, &mut ptb),
        }
        .map_err(|e| Error::InvalidArgument(format!("failed to build initial_record option: {e}")))?;
        let locking_config = locking_config.to_ptb(&mut ptb, audit_trail_package_id, tf_components_package_id)?;

        let immutable_metadata_tag = ImmutableMetadata::tag(audit_trail_package_id);

        let trail_metadata = match trail_metadata {
            Some(metadata) => {
                let metadata_arg = metadata.to_ptb(&mut ptb, audit_trail_package_id)?;
                tx::option_to_move(Some(metadata_arg), immutable_metadata_tag, &mut ptb)
                    .map_err(|e| Error::InvalidArgument(format!("failed to build trail_metadata option: {e}")))?
            }
            None => tx::option_to_move(None, immutable_metadata_tag, &mut ptb)
                .map_err(|e| Error::InvalidArgument(format!("failed to build trail_metadata option: {e}")))?,
        };

        let updatable_metadata = tx::ptb_pure(&mut ptb, "updatable_metadata", updatable_metadata)?;

        let record_tags = {
            let mut record_tags = record_tags.into_iter().collect::<Vec<_>>();
            record_tags.sort();
            tx::ptb_pure(&mut ptb, "record_tags", record_tags)?
        };
        let clock = tx::get_clock_ref(&mut ptb);

        let result = ptb.programmable_move_call(
            audit_trail_package_id,
            ident_str!("main").into(),
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

        let cap = match result {
            Argument::Result(idx) => Argument::NestedResult(idx, 0),
            _ => unreachable!("programmable_move_call should always return Argument::Result"),
        };
        ptb.transfer_arg(admin, cap);

        Ok(ptb.finish())
    }
}
