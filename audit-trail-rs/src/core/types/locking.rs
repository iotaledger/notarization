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
    pub delete_record_lock: LockingWindow,
}

impl LockingConfig {
    pub fn none() -> Self {
        Self {
            delete_record_lock: LockingWindow::none(),
        }
    }

    pub fn time_based(seconds: u64) -> Self {
        Self {
            delete_record_lock: LockingWindow::time_based(seconds),
        }
    }

    pub fn count_based(count: u64) -> Self {
        Self {
            delete_record_lock: LockingWindow::count_based(count),
        }
    }

    /// Creates a new `Argument` from the `LockingConfig`.
    ///
    /// To be used when creating or updating locking config on the ledger.
    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let delete_record_lock = self.delete_record_lock.to_ptb(ptb, package_id)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("locking").into(),
            ident_str!("new").into(),
            vec![],
            vec![delete_record_lock],
        ))
    }
}

/// Defines a locking window (time or count based).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LockingWindow {
    pub time_window_seconds: Option<u64>,
    pub count_window: Option<u64>,
}

impl LockingWindow {
    pub fn none() -> Self {
        Self {
            time_window_seconds: None,
            count_window: None,
        }
    }

    pub fn time_based(seconds: u64) -> Self {
        Self {
            time_window_seconds: Some(seconds),
            count_window: None,
        }
    }

    pub fn count_based(count: u64) -> Self {
        Self {
            time_window_seconds: None,
            count_window: Some(count),
        }
    }

    /// Creates a new `Argument` from the `LockingWindow`.
    ///
    /// To be used when creating or updating locking config on the ledger.
    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let time_window_seconds = utils::ptb_pure(ptb, "time_window_seconds", self.time_window_seconds)?;
        let count_window = utils::ptb_pure(ptb, "count_window", self.count_window)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("locking").into(),
            ident_str!("new_window").into(),
            vec![],
            vec![time_window_seconds, count_window],
        ))
    }
}
