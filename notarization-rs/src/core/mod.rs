use serde::{Deserialize, Serialize};

// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0
pub mod builder;
pub mod move_utils;
pub mod notarization;
pub mod operations;
pub mod state;
pub mod timelock;
/// Indicates the used Notarization method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotarizationMethod {
    Dynamic,
    Locked,
}
