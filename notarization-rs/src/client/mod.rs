// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Client implementations for interacting with notarizations on the IOTA blockchain.
//!
//! This module provides two client types:
//! - [`read_only`]: Read-only access to notarization data
//! - [`full_client`]: Full read-write access with transaction capabilities

pub mod full_client;
pub mod read_only;

pub use full_client::*;
pub use read_only::*;
