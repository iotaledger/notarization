// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

mod committee;
mod error;
mod proof;
mod source;
mod versioned;

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen::prelude::wasm_bindgen(typescript_custom_section)]
const LEDGER_SOURCE_IMPORT: &str = r#"
import type { LedgerSource } from "./source-types.js";
"#;
