// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

pub mod client;
pub mod core;
pub mod error;
pub(crate) mod iota_interaction_adapter;
pub(crate) mod package;

pub use client::full_client::NotarizationClient;
pub use client::read_only::NotarizationClientReadOnly;
/// HTTP utilities to implement the trait [HttpClient](product_common::http_client::HttpClient).
#[cfg(feature = "gas-station")]
pub use product_common::http_client;
