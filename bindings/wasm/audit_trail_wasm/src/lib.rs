// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![doc = include_str!("../README.md")]
#![warn(rustdoc::all)]
#![allow(deprecated)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::drop_non_drop)]
#![allow(clippy::unused_unit)]
#![allow(clippy::await_holding_refcell_ref)]

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

pub(crate) mod builder;
pub(crate) mod client;
pub(crate) mod client_read_only;
mod trail;
pub(crate) mod trail_handle;
pub(crate) mod types;

/// Shared wasm bindings re-exported from `product_common`.
pub use product_common::bindings::*;

/// Installs the panic hook used by the wasm bindings.
#[wasm_bindgen(start)]
pub fn start() -> std::result::Result<(), JsValue> {
    console_error_panic_hook::set_once();
    Ok(())
}

#[wasm_bindgen(typescript_custom_section)]
const CUSTOM_IMPORTS: &str = r#"
import {
  Transaction,
  TransactionOutput,
  TransactionBuilder,
  CoreClient,
  CoreClientReadOnly
} from '../lib/index';
"#;
