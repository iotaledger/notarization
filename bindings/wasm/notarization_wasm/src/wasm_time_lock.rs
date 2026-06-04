// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use notarization::core::types::TimeLock;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Discriminator for the variants of {@link TimeLock}.
///
/// @remarks
/// Returned by the {@link TimeLock.type} getter so callers can branch on the
/// kind of lock without inspecting its arguments.
#[wasm_bindgen(js_name = TimeLockType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmTimeLockType {
    /// No lock is applied.
    None = "None",
    /// Unlocks at a specific timestamp expressed in seconds since the Unix epoch.
    UnlockAt = "UnlockAt",
    /// Stays locked until the notarization is destroyed.
    /// Cannot be used for the `deleteLock` field of {@link LockMetadata}.
    UntilDestroyed = "UntilDestroyed",
}

/// A time-based lock applied to one of the lock fields of a notarization.
///
/// @remarks
/// Construct one with the static factory methods ({@link TimeLock.withUnlockAt},
/// {@link TimeLock.withUntilDestroyed}, {@link TimeLock.withNone}) and inspect it via
/// the {@link TimeLock.type} and {@link TimeLock.args} getters.
#[wasm_bindgen(js_name = TimeLock, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTimeLock(pub(crate) TimeLock);

#[wasm_bindgen(js_class = TimeLock)]
impl WasmTimeLock {
    /// Creates a lock that releases at a specific timestamp in seconds.
    ///
    /// @param timeSec - Unlock time, in seconds since the Unix epoch.
    ///
    /// @returns A {@link TimeLock} of type {@link TimeLockType.UnlockAt}.
    #[wasm_bindgen(js_name = withUnlockAt)]
    pub fn with_unlock_at(time_sec: u32) -> Self {
        Self(TimeLock::UnlockAt(time_sec))
    }

    /// Creates a lock that stays engaged until the notarization is destroyed.
    ///
    /// @remarks
    /// This variant is not valid for the `deleteLock` field of
    /// {@link LockMetadata} — using it there causes the on-chain transaction
    /// to abort.
    ///
    /// @returns A {@link TimeLock} of type {@link TimeLockType.UntilDestroyed}.
    #[wasm_bindgen(js_name = withUntilDestroyed)]
    pub fn with_until_destroyed() -> Self {
        Self(TimeLock::UntilDestroyed)
    }

    /// Creates an absent lock — semantically "no restriction".
    ///
    /// @returns A {@link TimeLock} of type {@link TimeLockType.None}.
    #[wasm_bindgen(js_name = withNone)]
    pub fn with_none() -> Self {
        Self(TimeLock::None)
    }

    /// The discriminator for which kind of lock this is.
    #[wasm_bindgen(js_name = "type", getter)]
    pub fn lock_type(&self) -> WasmTimeLockType {
        match &self.0 {
            TimeLock::UnlockAt(_) => WasmTimeLockType::UnlockAt,
            TimeLock::UntilDestroyed => WasmTimeLockType::UntilDestroyed,
            TimeLock::None => WasmTimeLockType::None,
        }
    }

    /// The argument carried by the lock variant, if any.
    ///
    /// @returns The unlock timestamp (`number`) for `UnlockAt` (seconds);
    /// `undefined` for `None` and `UntilDestroyed`.
    #[wasm_bindgen(js_name = "args", getter)]
    pub fn args(&self) -> JsValue {
        match &self.0 {
            TimeLock::UnlockAt(u) => JsValue::from(*u),
            _ => JsValue::UNDEFINED,
        }
    }
}
