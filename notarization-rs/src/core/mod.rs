// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Core notarization functionality and types.
//!
//! This module contains the fundamental building blocks for creating and managing
//! notarizations, including builders, state management, and transaction operations.

use serde::{Deserialize, Serialize};

pub mod builder;
pub mod move_utils;
pub mod operations;
pub mod transactions;
pub mod types;

/// Indicates the used Notarization method.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotarizationMethod {
    Dynamic,
    Locked,
}
