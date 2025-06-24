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

/// Indicates the used Notarization method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotarizationMethod {
    Dynamic,
    Locked,
}
