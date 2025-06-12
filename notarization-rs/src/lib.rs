// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod client;
pub mod client_tools;
pub mod core;
pub mod error;
pub(crate) mod iota_interaction_adapter;
pub mod package;

pub use client::read_only::NotarizationClientReadOnly;
pub use client::full_client::NotarizationClient;
