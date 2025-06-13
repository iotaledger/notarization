// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;
use std::time::SystemTime;

use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use iota_interaction::{MoveType, ident_str};
use serde::{Deserialize, Serialize};

use super::move_utils;
use crate::error::Error;

/// Metadata containing time-based access restrictions for a notarization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LockMetadata {
    pub update_lock: TimeLock,
    pub delete_lock: TimeLock,
    pub transfer_lock: TimeLock,
}

/// Represents different types of time-based locks that can be applied to
/// notarizations.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TimeLock {
    UnlockAt(u32),
    UntilDestroyed,
    None,
}

impl TimeLock {
    /// Creates a new `TimeLock` with a specified unlock time.\
    ///
    /// The unlock time is the time in seconds since the Unix epoch and
    /// must be in the future.
    pub fn new_with_ts(unlock_time: u32) -> Result<Self, Error> {
        if unlock_time
            <= SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time is before the Unix epoch")
                .as_secs() as u32
        {
            return Err(Error::InvalidArgument("unlock time must be in the future".to_string()));
        }

        Ok(TimeLock::UnlockAt(unlock_time))
    }
    /// Creates a new `Argument` from the `TimeLock`.
    ///
    /// To be used when creating a new `Notarization` object on the ledger.
    pub(super) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        match self {
            TimeLock::UnlockAt(unlock_time) => new_unlock_at(ptb, *unlock_time, package_id),
            TimeLock::UntilDestroyed => new_until_destroyed(ptb, package_id),
            TimeLock::None => new_none(ptb, package_id),
        }
    }
}

pub(super) fn new_unlock_at(ptb: &mut Ptb, unlock_time: u32, package_id: ObjectID) -> Result<Argument, Error> {
    let clock = move_utils::get_clock_ref(ptb);
    let unlock_time = move_utils::ptb_pure(ptb, "unlock_time", unlock_time)?;

    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("timelock").into(),
        ident_str!("unlock_at").into(),
        vec![],
        vec![unlock_time, clock],
    ))
}

pub(super) fn new_until_destroyed(ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("timelock").into(),
        ident_str!("until_destroyed").into(),
        vec![],
        vec![],
    ))
}

pub(super) fn new_none(ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("timelock").into(),
        ident_str!("none").into(),
        vec![],
        vec![],
    ))
}

impl MoveType for TimeLock {
    fn move_type(package: ObjectID) -> TypeTag {
        TypeTag::from_str(format!("{}::timelock::TimeLock", package).as_str()).expect("failed to create type tag")
    }
}
