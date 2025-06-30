// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! # TimeLock
//!
//! This module defines the time-based locks for notarizations.
//!
//! ## Overview
//!
//! The time-based locks are used to restrict the access to a notarization.
//!
//! ## Types
//!
//! - `UnlockAt`: The lock is unlocked at a specific time.
//! - `UntilDestroyed`: The lock is unlocked when the notarization is destroyed.
//! - `None`: The lock is not applied.

use std::str::FromStr;
use std::time::SystemTime;

use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use iota_interaction::{MoveType, ident_str};
use serde::{Deserialize, Serialize};

use super::super::move_utils;
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
    /// A lock that is unlocked at a specific time.
    UnlockAt(u32),
    /// A lock that is unlocked when the notarization is destroyed.
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
    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        match self {
            TimeLock::UnlockAt(unlock_time) => new_unlock_at(ptb, *unlock_time, package_id),
            TimeLock::UntilDestroyed => new_until_destroyed(ptb, package_id),
            TimeLock::None => new_none(ptb, package_id),
        }
    }
}

/// Creates a new `Argument` for the `unlock_at` function.
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

/// Creates a new `Argument` for the `until_destroyed` function.
pub(super) fn new_until_destroyed(ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
    Ok(ptb.programmable_move_call(
        package_id,
        ident_str!("timelock").into(),
        ident_str!("until_destroyed").into(),
        vec![],
        vec![],
    ))
}

/// Creates a new `Argument` for the `none` function.
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
        TypeTag::from_str(format!("{package}::timelock::TimeLock").as_str()).expect("failed to create type tag")
    }
}
