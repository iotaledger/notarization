// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::ident_str;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use serde::{Deserialize, Serialize};

use crate::core::utils;
use crate::error::Error;

/// Locking configuration for the audit trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LockingConfig {
    pub delete_record: LockingWindow,
}

impl LockingConfig {
    /// Creates a new `Argument` from the `LockingConfig`.
    ///
    /// To be used when creating or updating locking config on the ledger.
    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let delete_record_lock = self.delete_record.to_ptb(ptb, package_id)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("locking").into(),
            ident_str!("new").into(),
            vec![],
            vec![delete_record_lock],
        ))
    }
}

/// Defines a locking window (none, time based, or count based).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LockingWindow {
    #[default]
    None,
    TimeBased {
        seconds: u64,
    },
    CountBased {
        count: u64,
    },
}

impl LockingWindow {
    /// Creates a new `Argument` from the `LockingWindow`.
    ///
    /// To be used when creating or updating locking config on the ledger.
    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        match self {
            Self::None => Ok(ptb.programmable_move_call(
                package_id,
                ident_str!("locking").into(),
                ident_str!("window_none").into(),
                vec![],
                vec![],
            )),
            Self::TimeBased { seconds } => {
                let seconds = utils::ptb_pure(ptb, "seconds", *seconds)?;
                Ok(ptb.programmable_move_call(
                    package_id,
                    ident_str!("locking").into(),
                    ident_str!("window_time_based").into(),
                    vec![],
                    vec![seconds],
                ))
            }
            Self::CountBased { count } => {
                let count = utils::ptb_pure(ptb, "count", *count)?;
                Ok(ptb.programmable_move_call(
                    package_id,
                    ident_str!("locking").into(),
                    ident_str!("window_count_based").into(),
                    vec![],
                    vec![count],
                ))
            }
        }
    }
}
