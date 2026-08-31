// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::string::FromUtf8Error;

use iota_sdk_types::{AddressParseError, DigestParseError};
use poi_rs::{
    CommitteeResolutionError, CommitteeResolutionErrorKind, ProofBuilderError, ProofVerificationError,
    SerializationError, SourceError, VerifyError,
};
use wasm_bindgen::{JsCast, JsValue};

pub type WasmResult<T> = Result<T, WasmError>;

#[derive(Debug, thiserror::Error)]
pub enum WasmError {
    #[error(transparent)]
    Verify(#[from] VerifyError),
    #[error(transparent)]
    ProofVerification(#[from] ProofVerificationError),
    #[error(transparent)]
    CommitteeResolution(#[from] CommitteeResolutionError),
    #[error(transparent)]
    ProofBuilder(#[from] ProofBuilderError),
    #[error(transparent)]
    Source(#[from] SourceError),
    #[error(transparent)]
    Serialization(#[from] SerializationError),
    #[error(transparent)]
    Poi(#[from] PoiError),
    #[error(transparent)]
    Address(#[from] AddressParseError),
    #[error(transparent)]
    Digest(#[from] DigestParseError),
    #[error(transparent)]
    Bcs(#[from] bcs::Error),
    #[error(transparent)]
    Utf8(#[from] FromUtf8Error),
}

#[derive(Clone, Copy, Debug)]
enum WasmErrorCode {
    ProofInvalid,
    CommitteeResolution,
    SourceRequest,
    NotFound,
    InvalidInput,
    Internal,
}

impl WasmErrorCode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::ProofInvalid => "PROOF_INVALID",
            Self::CommitteeResolution => "COMMITTEE_RESOLUTION",
            Self::SourceRequest => "SOURCE_REQUEST",
            Self::NotFound => "NOT_FOUND",
            Self::InvalidInput => "INVALID_INPUT",
            Self::Internal => "INTERNAL",
        }
    }
}

impl WasmError {
    fn code(&self) -> WasmErrorCode {
        match self {
            Self::Verify(_) => WasmErrorCode::ProofInvalid,
            Self::ProofVerification(error) => match error {
                ProofVerificationError::CommitteeResolution { source } => committee_resolution_code(source),
                ProofVerificationError::Proof { .. } => WasmErrorCode::ProofInvalid,
                _ => WasmErrorCode::Internal,
            },
            Self::CommitteeResolution(error) => committee_resolution_code(error),
            Self::ProofBuilder(error) => match error {
                ProofBuilderError::Source { .. } => WasmErrorCode::SourceRequest,
                ProofBuilderError::TransactionNotFound { .. }
                | ProofBuilderError::ObjectNotFound { .. }
                | ProofBuilderError::EventNotFound { .. } => WasmErrorCode::NotFound,
                ProofBuilderError::MissingRequest
                | ProofBuilderError::ObjectReferenceMismatch { .. }
                | ProofBuilderError::ObjectNotChangedByTransaction { .. }
                | ProofBuilderError::TransactionMismatch { .. } => WasmErrorCode::InvalidInput,
                _ => WasmErrorCode::Internal,
            },
            Self::Source(_) => WasmErrorCode::SourceRequest,
            Self::Serialization(_) | Self::Address(_) | Self::Digest(_) => WasmErrorCode::InvalidInput,
            Self::Poi(error) => match error {
                PoiError::JavaScript(_) => WasmErrorCode::SourceRequest,
                PoiError::InvalidInput(_) => WasmErrorCode::InvalidInput,
                PoiError::InvalidResponse(_) => WasmErrorCode::Internal,
            },
            Self::Bcs(_) | Self::Utf8(_) => WasmErrorCode::Internal,
        }
    }
}

impl From<WasmError> for JsValue {
    fn from(error: WasmError) -> Self {
        let code = error.code().as_str();
        let mut message = error.to_string();
        let mut source = error.source();

        while let Some(cause) = source {
            message.push_str(": ");
            message.push_str(&cause.to_string());
            source = cause.source();
        }

        let js_error = js_sys::Error::new(&message);
        js_error.set_name("PoiError");
        let _ = js_sys::Reflect::set(js_error.as_ref(), &JsValue::from_str("code"), &JsValue::from_str(code));

        js_error.into()
    }
}

fn committee_resolution_code(error: &CommitteeResolutionError) -> WasmErrorCode {
    match &error.kind {
        CommitteeResolutionErrorKind::FetchCommittee { .. }
        | CommitteeResolutionErrorKind::FetchCurrentEpoch { .. }
        | CommitteeResolutionErrorKind::FetchEpochHistory { .. } => WasmErrorCode::SourceRequest,
        _ => WasmErrorCode::CommitteeResolution,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PoiError {
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
