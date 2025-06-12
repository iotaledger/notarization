// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use notarization::core::timelock::TimeLock;


#[wasm_bindgen(js_name = TimeLockType)]
pub enum WasmTimeLockType {
    None = "None",
    UnlockAt = "UnlockAt",
    UntilDestroyed = "UntilDestroyed",
}

#[wasm_bindgen(js_name = TimeLock, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTimeLock(pub(crate) TimeLock);

#[wasm_bindgen(js_class = TimeLock)]
impl WasmTimeLock {
    #[wasm_bindgen(js_name = withUnlockAt)]
    pub fn with_unlock_at(time: u32) -> Self {
        Self(TimeLock::UnlockAt(time))
    }
    
    #[wasm_bindgen(js_name = withUntilDestroyed)]
    pub fn with_until_destroyed() -> Self {
        Self(TimeLock::UntilDestroyed)
    }

    #[wasm_bindgen(js_name = withNone)]
    pub fn with_none() -> Self {
        Self(TimeLock::None)
    }

    #[wasm_bindgen(js_name = "type", getter)]
    pub fn lock_type(&self) -> WasmTimeLockType {
        match &self.0 {
            TimeLock::UnlockAt(_) => WasmTimeLockType::UnlockAt,
            TimeLock::UntilDestroyed => WasmTimeLockType::UntilDestroyed,
            TimeLock::None => WasmTimeLockType::None,
        }
    }

    #[wasm_bindgen(js_name = "args", getter)]
    pub fn args(&self) -> JsValue {
        match &self.0 {
            TimeLock::UnlockAt(u) => JsValue::from(*u),
            _ => JsValue::UNDEFINED,
        }
    }
}