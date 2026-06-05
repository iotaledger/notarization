// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Core data types for notarization.

pub mod event;
pub mod metadata;
pub mod notarization;
pub mod state;
pub mod timelock;

pub use event::*;
pub use metadata::*;
pub use notarization::*;
use serde::{Deserialize, Serialize};
pub use state::*;
pub use timelock::*;

/// Identifies the Notarization Method of a notarization.
///
/// The Notarization Method is fixed at creation and determines which
/// operations are permitted on the notarization afterwards. The set of
/// Notarization Methods is closed in the current version of the package but
/// may be extended in future versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotarizationMethod {
    /// Method whose `state` and `updatable_metadata` can be updated after
    /// creation and which may optionally be transfer-locked.
    Dynamic,
    /// Method whose `state` and `updatable_metadata` are immutable after
    /// creation and whose destruction is gated by a `delete_lock`.
    Locked,
}
