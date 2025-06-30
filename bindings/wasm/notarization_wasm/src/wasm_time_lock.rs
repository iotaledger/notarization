// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use notarization::core::types::TimeLock;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Represents the type of a time lock.
///
/// This enum defines the possible types of time locks that can be applied to a notarization object.
/// - `None`: No time lock is applied.
/// - `UnlockAt`: The object will unlock at a specific timestamp.
/// - `UntilDestroyed`: The object remains locked until it is destroyed.
#[wasm_bindgen(js_name = TimeLockType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmTimeLockType {
    None = "None",
    UnlockAt = "UnlockAt",
    UntilDestroyed = "UntilDestroyed",
}

/// Represents a time lock configuration.
///
/// It allows the creation and inspection of time lock configurations for notarization objects.
#[wasm_bindgen(js_name = TimeLock, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTimeLock(pub(crate) TimeLock);

#[wasm_bindgen(js_class = TimeLock)]
impl WasmTimeLock {
    /// Creates a time lock that unlocks at a specific timestamp.
    ///
    /// # Arguments
    /// * `time` - The timestamp in seconds since the Unix epoch at which the object will unlock.
    ///
    /// # Returns
    /// A new `TimeLock` instance configured to unlock at the specified timestamp.
    #[wasm_bindgen(js_name = withUnlockAt)]
    pub fn with_unlock_at(time: u32) -> Self {
        Self(TimeLock::UnlockAt(time))
    }

    /// Creates a time lock that remains locked until the object is destroyed.
    ///
    /// # Returns
    /// A new `TimeLock` instance configured to remain locked until destruction.
    #[wasm_bindgen(js_name = withUntilDestroyed)]
    pub fn with_until_destroyed() -> Self {
        Self(TimeLock::UntilDestroyed)
    }

    /// Creates a time lock with no restrictions.
    ///
    /// # Returns
    /// A new `TimeLock` instance with no time lock applied.
    #[wasm_bindgen(js_name = withNone)]
    pub fn with_none() -> Self {
        Self(TimeLock::None)
    }

    /// Retrieves the type of the time lock.
    ///
    /// # Returns
    /// The `TimeLockType` representing the type of the time lock.
    #[wasm_bindgen(js_name = "type", getter)]
    pub fn lock_type(&self) -> WasmTimeLockType {
        match &self.0 {
            TimeLock::UnlockAt(_) => WasmTimeLockType::UnlockAt,
            TimeLock::UntilDestroyed => WasmTimeLockType::UntilDestroyed,
            TimeLock::None => WasmTimeLockType::None,
        }
    }

    /// Retrieves the arguments associated with the time lock.
    ///
    /// # Returns
    /// An `any` value containing the arguments for the time lock:
    /// - For `UnlockAt`, the timestamp is returned.
    /// - For other types, `undefined` is returned.
    #[wasm_bindgen(js_name = "args", getter)]
    pub fn args(&self) -> JsValue {
        match &self.0 {
            TimeLock::UnlockAt(u) => JsValue::from(*u),
            _ => JsValue::UNDEFINED,
        }
    }
}
