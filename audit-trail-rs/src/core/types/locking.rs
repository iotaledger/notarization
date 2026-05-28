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
///
/// Combines three independent rules: a per-record delete window, a trail-delete
/// time lock, and a write time lock. Two invariants apply:
///
/// - `delete_trail_lock` must not be [`TimeLock::UntilDestroyed`]; that variant
///   is reserved for `write_lock`.
/// - `delete_record_window`, when [`LockingWindow::CountBased`], must use
///   `count > 0`; use [`LockingWindow::None`] to express "no deletion lock".
///
/// Public entry points that accept a `LockingConfig` call [`LockingConfig::validate`]
/// up front, so misconfiguration is reported client-side before any transaction
/// is built; the same invariants are enforced on-chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LockingConfig {
    /// Delete-window policy applied to individual records. Records that fall
    /// inside the window are locked against deletion. A [`LockingWindow::CountBased`]
    /// window must use `count > 0`.
    pub delete_record_window: LockingWindow,
    /// Time lock that gates deletion of the entire trail. Must not be
    /// [`TimeLock::UntilDestroyed`].
    pub delete_trail_lock: TimeLock,
    /// Time lock that gates record writes (`add_record`).
    pub write_lock: TimeLock,
}

impl LockingConfig {
    /// Validates the locking configuration without contacting the chain.
    ///
    /// Currently this rejects:
    /// - [`LockingWindow::CountBased`] with `count == 0` (mirrors the Move `ECountWindowMustBePositive` abort).
    /// - [`TimeLock::UntilDestroyed`] used as `delete_trail_lock` (mirrors the Move
    ///   `EUntilDestroyedNotSupportedForDeleteTrail` abort). `write_lock` may still be `UntilDestroyed`.
    ///
    /// Public entry points that accept a `LockingConfig` call this so that misconfiguration is reported
    /// before any transaction is built.
    pub fn validate(&self) -> Result<(), Error> {
        self.delete_record_window.validate()?;
        self.delete_trail_lock.validate_as_delete_trail_lock()?;
        Ok(())
    }

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
/// `UntilDestroyed` is rejected by the audit-trail package when used for the
/// trail-delete lock; pass it only for the write lock.
///
/// Must match `tf_components::timelock::TimeLock` variant order for BCS compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TimeLock {
    /// Unlocks at the given Unix timestamp in seconds.
    UnlockAt(u32),
    /// Unlocks at the given Unix timestamp in milliseconds.
    UnlockAtMs(u64),
    /// Remains locked until the protected object is explicitly destroyed.
    /// Not supported as the trail-delete lock.
    UntilDestroyed,
    /// Represents an always-locked state.
    Infinite,
    /// Disables the time lock.
    #[default]
    None,
}

impl TimeLock {
    /// Validates this lock as a candidate for the trail-level delete lock.
    ///
    /// Rejects [`TimeLock::UntilDestroyed`] (mirrors the Move
    /// `EUntilDestroyedNotSupportedForDeleteTrail` abort). All other variants are accepted; time-based
    /// timestamp validity is enforced on-chain because it depends on the clock at execution time.
    pub fn validate_as_delete_trail_lock(&self) -> Result<(), Error> {
        if matches!(self, Self::UntilDestroyed) {
            return Err(Error::InvalidArgument(
                "TimeLock::UntilDestroyed is not supported as a delete-trail lock".to_string(),
            ));
        }
        Ok(())
    }

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

/// Defines a delete-record locking window.
///
/// A window describes the period during which a record is *locked against
/// deletion*. Records outside the window may be deleted (subject to the
/// remaining permission and tag checks).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LockingWindow {
    /// No delete window is enforced; records may be deleted at any time.
    #[default]
    None,
    /// A record is locked against deletion while its age (in milliseconds)
    /// is below the configured number of seconds since it was added.
    TimeBased {
        /// Window size in seconds. Records younger than this are locked.
        seconds: u64,
    },
    /// Locks the last `count` records currently present in trail order.
    ///
    /// The protected window is evaluated against the records present when the
    /// transaction begins; concurrent additions are observed by subsequent
    /// transactions only. `count` must be positive — use [`LockingWindow::None`]
    /// to express "no deletion lock". Constructing this variant with `count == 0`
    /// is rejected client-side with [`Error::InvalidArgument`] and would otherwise
    /// abort on-chain with `ECountWindowMustBePositive`.
    ///
    /// The on-chain check walks backward from the current tail once per call,
    /// so delete gas scales linearly with `count`.
    CountBased {
        /// Number of current tail records protected from deletion. Must be `> 0`.
        count: u64,
    },
}

impl LockingWindow {
    /// Validates the window configuration without contacting the chain.
    ///
    /// Rejects [`LockingWindow::CountBased`] with `count == 0` (mirrors the Move
    /// `ECountWindowMustBePositive` abort). All other variants are always valid.
    pub fn validate(&self) -> Result<(), Error> {
        if let Self::CountBased { count: 0 } = self {
            return Err(Error::InvalidArgument(
                "LockingWindow::CountBased requires count > 0; use LockingWindow::None for no deletion lock"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Creates a new `Argument` from the `LockingWindow`.
    ///
    /// To be used when creating or updating locking config on the ledger.
    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        self.validate()?;
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
