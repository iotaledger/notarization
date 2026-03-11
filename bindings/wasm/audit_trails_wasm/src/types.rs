// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use audit_trails::core::types::{
    Data, ImmutableMetadata, LockingConfig, LockingWindow, PaginatedRecord, Record, RecordCorrection, TimeLock,
};
use js_sys::Uint8Array;
use product_common::bindings::WasmIotaAddress;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Empty, inspectable)]
pub struct WasmEmpty;

impl From<()> for WasmEmpty {
    fn from(_: ()) -> Self {
        Self
    }
}

#[wasm_bindgen(js_name = Data, inspectable)]
#[derive(Clone)]
pub struct WasmData(pub(crate) Data);

#[wasm_bindgen(js_class = Data)]
impl WasmData {
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> JsValue {
        match &self.0 {
            Data::Bytes(bytes) => Uint8Array::from(bytes.as_slice()).into(),
            Data::Text(text) => JsValue::from(text),
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        match &self.0 {
            Data::Bytes(bytes) => String::from_utf8_lossy(bytes).to_string(),
            Data::Text(text) => text.clone(),
        }
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        match &self.0 {
            Data::Bytes(bytes) => bytes.clone(),
            Data::Text(text) => text.as_bytes().to_vec(),
        }
    }

    #[wasm_bindgen(js_name = fromString)]
    pub fn from_string(data: String) -> Self {
        Self(Data::text(data))
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: Uint8Array) -> Self {
        Self(Data::bytes(data.to_vec()))
    }
}

impl From<Data> for WasmData {
    fn from(value: Data) -> Self {
        Self(value)
    }
}

impl From<WasmData> for Data {
    fn from(value: WasmData) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_name = TimeLockType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmTimeLockType {
    None,
    UnlockAt,
    UnlockAtMs,
    UntilDestroyed,
    Infinite,
}

#[wasm_bindgen(js_name = TimeLock, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTimeLock(pub(crate) TimeLock);

#[wasm_bindgen(js_class = TimeLock)]
impl WasmTimeLock {
    #[wasm_bindgen(js_name = withUnlockAt)]
    pub fn with_unlock_at(time_sec: u32) -> Self {
        Self(TimeLock::UnlockAt(time_sec))
    }

    #[wasm_bindgen(js_name = withUnlockAtMs)]
    pub fn with_unlock_at_ms(time_ms: u64) -> Self {
        Self(TimeLock::UnlockAtMs(time_ms))
    }

    #[wasm_bindgen(js_name = withUntilDestroyed)]
    pub fn with_until_destroyed() -> Self {
        Self(TimeLock::UntilDestroyed)
    }

    #[wasm_bindgen(js_name = withInfinite)]
    pub fn with_infinite() -> Self {
        Self(TimeLock::Infinite)
    }

    #[wasm_bindgen(js_name = withNone)]
    pub fn with_none() -> Self {
        Self(TimeLock::None)
    }

    #[wasm_bindgen(js_name = "type", getter)]
    pub fn lock_type(&self) -> WasmTimeLockType {
        match self.0 {
            TimeLock::None => WasmTimeLockType::None,
            TimeLock::UnlockAt(_) => WasmTimeLockType::UnlockAt,
            TimeLock::UnlockAtMs(_) => WasmTimeLockType::UnlockAtMs,
            TimeLock::UntilDestroyed => WasmTimeLockType::UntilDestroyed,
            TimeLock::Infinite => WasmTimeLockType::Infinite,
        }
    }

    #[wasm_bindgen(js_name = "args", getter)]
    pub fn args(&self) -> JsValue {
        match self.0 {
            TimeLock::UnlockAt(value) => JsValue::from(value),
            TimeLock::UnlockAtMs(value) => JsValue::from(value),
            _ => JsValue::UNDEFINED,
        }
    }
}

impl From<TimeLock> for WasmTimeLock {
    fn from(value: TimeLock) -> Self {
        Self(value)
    }
}

impl From<WasmTimeLock> for TimeLock {
    fn from(value: WasmTimeLock) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_name = LockingWindowType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmLockingWindowType {
    None,
    TimeBased,
    CountBased,
}

#[wasm_bindgen(js_name = LockingWindow, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmLockingWindow(pub(crate) LockingWindow);

#[wasm_bindgen(js_class = LockingWindow)]
impl WasmLockingWindow {
    #[wasm_bindgen(js_name = withNone)]
    pub fn with_none() -> Self {
        Self(LockingWindow::None)
    }

    #[wasm_bindgen(js_name = withTimeBased)]
    pub fn with_time_based(seconds: u64) -> Self {
        Self(LockingWindow::TimeBased { seconds })
    }

    #[wasm_bindgen(js_name = withCountBased)]
    pub fn with_count_based(count: u64) -> Self {
        Self(LockingWindow::CountBased { count })
    }

    #[wasm_bindgen(js_name = "type", getter)]
    pub fn window_type(&self) -> WasmLockingWindowType {
        match self.0 {
            LockingWindow::None => WasmLockingWindowType::None,
            LockingWindow::TimeBased { .. } => WasmLockingWindowType::TimeBased,
            LockingWindow::CountBased { .. } => WasmLockingWindowType::CountBased,
        }
    }

    #[wasm_bindgen(js_name = "args", getter)]
    pub fn args(&self) -> JsValue {
        match self.0 {
            LockingWindow::TimeBased { seconds } => JsValue::from(seconds),
            LockingWindow::CountBased { count } => JsValue::from(count),
            LockingWindow::None => JsValue::UNDEFINED,
        }
    }
}

impl From<LockingWindow> for WasmLockingWindow {
    fn from(value: LockingWindow) -> Self {
        Self(value)
    }
}

impl From<WasmLockingWindow> for LockingWindow {
    fn from(value: WasmLockingWindow) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_name = LockingConfig, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmLockingConfig {
    #[wasm_bindgen(js_name = deleteRecordWindow)]
    pub delete_record_window: WasmLockingWindow,
    #[wasm_bindgen(js_name = deleteTrailLock)]
    pub delete_trail_lock: WasmTimeLock,
    #[wasm_bindgen(js_name = writeLock)]
    pub write_lock: WasmTimeLock,
}

#[wasm_bindgen(js_class = LockingConfig)]
impl WasmLockingConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(
        delete_record_window: WasmLockingWindow,
        delete_trail_lock: WasmTimeLock,
        write_lock: WasmTimeLock,
    ) -> Self {
        Self {
            delete_record_window,
            delete_trail_lock,
            write_lock,
        }
    }
}

impl From<LockingConfig> for WasmLockingConfig {
    fn from(value: LockingConfig) -> Self {
        Self {
            delete_record_window: value.delete_record_window.into(),
            delete_trail_lock: value.delete_trail_lock.into(),
            write_lock: value.write_lock.into(),
        }
    }
}

impl From<WasmLockingConfig> for LockingConfig {
    fn from(value: WasmLockingConfig) -> Self {
        Self {
            delete_record_window: value.delete_record_window.into(),
            delete_trail_lock: value.delete_trail_lock.into(),
            write_lock: value.write_lock.into(),
        }
    }
}

#[wasm_bindgen(js_name = ImmutableMetadata, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmImmutableMetadata {
    pub name: String,
    pub description: Option<String>,
}

impl From<ImmutableMetadata> for WasmImmutableMetadata {
    fn from(value: ImmutableMetadata) -> Self {
        Self {
            name: value.name,
            description: value.description,
        }
    }
}

impl From<WasmImmutableMetadata> for ImmutableMetadata {
    fn from(value: WasmImmutableMetadata) -> Self {
        ImmutableMetadata {
            name: value.name,
            description: value.description,
        }
    }
}

#[wasm_bindgen(js_name = RecordCorrection, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmRecordCorrection {
    pub replaces: Vec<u64>,
    #[wasm_bindgen(js_name = isReplacedBy)]
    pub is_replaced_by: Option<u64>,
}

impl From<RecordCorrection> for WasmRecordCorrection {
    fn from(value: RecordCorrection) -> Self {
        let mut replaces: Vec<u64> = value.replaces.into_iter().collect();
        replaces.sort_unstable();
        Self {
            replaces,
            is_replaced_by: value.is_replaced_by,
        }
    }
}

impl From<WasmRecordCorrection> for RecordCorrection {
    fn from(value: WasmRecordCorrection) -> Self {
        Self {
            replaces: value.replaces.into_iter().collect::<HashSet<_>>(),
            is_replaced_by: value.is_replaced_by,
        }
    }
}

#[wasm_bindgen(js_name = Record, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRecord {
    pub data: WasmData,
    pub metadata: Option<String>,
    #[wasm_bindgen(js_name = sequenceNumber)]
    pub sequence_number: u64,
    #[wasm_bindgen(js_name = addedBy)]
    pub added_by: WasmIotaAddress,
    #[wasm_bindgen(js_name = addedAt)]
    pub added_at: u64,
    pub correction: WasmRecordCorrection,
}

impl From<Record<Data>> for WasmRecord {
    fn from(value: Record<Data>) -> Self {
        Self {
            data: value.data.into(),
            metadata: value.metadata,
            sequence_number: value.sequence_number,
            added_by: value.added_by.to_string(),
            added_at: value.added_at,
            correction: value.correction.into(),
        }
    }
}

#[wasm_bindgen(js_name = PaginatedRecord, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmPaginatedRecord {
    pub records: Vec<WasmRecord>,
    #[wasm_bindgen(js_name = nextCursor)]
    pub next_cursor: Option<u64>,
    #[wasm_bindgen(js_name = hasNextPage)]
    pub has_next_page: bool,
}

impl From<PaginatedRecord<Data>> for WasmPaginatedRecord {
    fn from(value: PaginatedRecord<Data>) -> Self {
        Self {
            records: value.records.into_values().map(Into::into).collect(),
            next_cursor: value.next_cursor,
            has_next_page: value.has_next_page,
        }
    }
}
