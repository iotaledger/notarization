// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Core notarization functionality and types.
//!
//! This module contains the fundamental building blocks for creating and managing
//! notarizations, including builders, state management, and transaction operations.

pub mod builder;
pub(crate) mod move_utils;
pub(crate) mod operations;
pub mod transactions;
pub mod types;
