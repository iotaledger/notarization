// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use wasm_bindgen::{JsCast, JsValue};

#[derive(Debug, thiserror::Error)]
pub(crate) enum PoiError {
    #[error("{0}")]
    JavaScript(String),
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    InvalidResponse(String),
}

impl PoiError {
    pub(crate) fn from_js(value: JsValue) -> Self {
        let message = value
            .dyn_ref::<js_sys::Error>()
            .map(js_sys::Error::message)
            .and_then(|message| message.as_string())
            .or_else(|| value.as_string())
            .unwrap_or_else(|| format!("{value:?}"));

        Self::JavaScript(message)
    }

    pub(crate) fn invalid_response(message: impl Into<String>) -> Self {
        Self::InvalidResponse(message.into())
    }

    pub(crate) fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }
}

pub(crate) trait WasmResult<T> {
    fn wasm_result(self) -> Result<T, JsValue>;
}

impl<T, E> WasmResult<T> for Result<T, E>
where
    E: Error,
{
    fn wasm_result(self) -> Result<T, JsValue> {
        self.map_err(|error| {
            let mut message = error.to_string();
            let mut source = error.source();

            while let Some(cause) = source {
                message.push_str(": ");
                message.push_str(&cause.to_string());
                source = cause.source();
            }

            js_sys::Error::new(&message).into()
        })
    }
}
