// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

#![allow(deprecated)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::drop_non_drop)]
#![allow(clippy::unused_unit)]
#![allow(clippy::await_holding_refcell_ref)]

use std::borrow::Cow;

use iota_interaction_ts::wasm_error::{Result, WasmError};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

mod trail;
pub(crate) mod builder;
pub(crate) mod client;
pub(crate) mod client_read_only;
pub(crate) mod trail_handle;
pub(crate) mod trail_records;
pub(crate) mod types;

pub use product_common::bindings::*;

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

pub(crate) fn audit_trails_wasm_error(error: audit_trails::error::Error) -> JsValue {
    JsValue::from(WasmError {
        name: Cow::Borrowed("audit_trails::Error"),
        message: Cow::Owned(error.to_string()),
    })
}

pub(crate) fn audit_trails_wasm_result<T>(
    result: std::result::Result<T, audit_trails::error::Error>,
) -> Result<T> {
    result.map_err(audit_trails_wasm_error)
}
