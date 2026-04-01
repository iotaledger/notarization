// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::ident_str;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use serde::{Deserialize, Serialize};

use crate::core::internal::tx;
use crate::error::Error;

/// Locking configuration for the audit trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LockingConfig {
    pub delete_record_window: LockingWindow,
    pub delete_trail_lock: TimeLock,
    pub write_lock: TimeLock,
}

impl LockingConfig {
    /// Creates a new `Argument` from the `LockingConfig`.
    ///
    /// To be used when creating or updating locking config on the ledger.
    pub(in crate::core) fn to_ptb(
        &self,
        ptb: &mut Ptb,
        package_id: ObjectID,
        tf_components_package_id: ObjectID,
    ) -> Result<Argument, Error> {
        let delete_record_window = self.delete_record_window.to_ptb(ptb, package_id)?;
        let delete_trail_lock = self.delete_trail_lock.to_ptb(ptb, tf_components_package_id)?;
        let write_lock = self.write_lock.to_ptb(ptb, tf_components_package_id)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("locking").into(),
            ident_str!("new").into(),
            vec![],
            vec![delete_record_window, delete_trail_lock, write_lock],
        ))
    }
}

/// Time-based lock for trail-level operations.
///
/// Must match `tf_components::timelock::TimeLock` variant order for BCS compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TimeLock {
    UnlockAt(u32),
    UnlockAtMs(u64),
    UntilDestroyed,
    Infinite,
    #[default]
    None,
}

impl TimeLock {
    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        match self {
            Self::None => Ok(ptb.programmable_move_call(
                package_id,
                ident_str!("timelock").into(),
                ident_str!("none").into(),
                vec![],
                vec![],
            )),
            Self::Infinite => Ok(ptb.programmable_move_call(
                package_id,
                ident_str!("timelock").into(),
                ident_str!("infinite").into(),
                vec![],
                vec![],
            )),
            Self::UntilDestroyed => Ok(ptb.programmable_move_call(
                package_id,
                ident_str!("timelock").into(),
                ident_str!("until_destroyed").into(),
                vec![],
                vec![],
            )),
            Self::UnlockAt(unix_time) => {
                let unix_time = tx::ptb_pure(ptb, "unix_time", *unix_time)?;
                let clock = tx::get_clock_ref(ptb);

                Ok(ptb.programmable_move_call(
                    package_id,
                    ident_str!("timelock").into(),
                    ident_str!("unlock_at").into(),
                    vec![],
                    vec![unix_time, clock],
                ))
            }
            Self::UnlockAtMs(unix_time_ms) => {
                let unix_time_ms = tx::ptb_pure(ptb, "unix_time_ms", *unix_time_ms)?;
                let clock = tx::get_clock_ref(ptb);

                Ok(ptb.programmable_move_call(
                    package_id,
                    ident_str!("timelock").into(),
                    ident_str!("unlock_at_ms").into(),
                    vec![],
                    vec![unix_time_ms, clock],
                ))
            }
        }
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
                let seconds = tx::ptb_pure(ptb, "seconds", *seconds)?;
                Ok(ptb.programmable_move_call(
                    package_id,
                    ident_str!("locking").into(),
                    ident_str!("window_time_based").into(),
                    vec![],
                    vec![seconds],
                ))
            }
            Self::CountBased { count } => {
                let count = tx::ptb_pure(ptb, "count", *count)?;
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
