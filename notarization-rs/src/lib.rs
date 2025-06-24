// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod client;
pub mod core;
pub mod error;
pub(crate) mod iota_interaction_adapter;
pub(crate) mod package;

pub use client::full_client::NotarizationClient;
pub use client::read_only::NotarizationClientReadOnly;
