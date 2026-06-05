// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Platform-dependent adapter re-exports for the underlying IOTA interaction layer.
//!
//! This keeps the rest of the crate generic over native and wasm targets by exposing the same
//! adapter names from either `iota_interaction_rust` or `iota_interaction_ts`.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use iota_interaction_rust::*;
#[cfg(target_arch = "wasm32")]
pub(crate) use iota_interaction_ts::*;
