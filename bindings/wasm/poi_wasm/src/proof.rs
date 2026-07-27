// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

use iota_sdk_types::ObjectId;
use iota_types::{digests::TransactionDigest, event::EventID};
use js_sys::Uint8Array;
use poi_rs::{Proof, ProofBuilder};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::committee::WasmCommittee;
use crate::source::{LedgerSource, SourceAdapter};

/// Proof of Inclusion evidence constructed by `poi-rs`.
#[wasm_bindgen(js_name = Proof)]
pub struct WasmProof(pub(crate) Proof);

#[wasm_bindgen(js_class = Proof)]
impl WasmProof {
    /// Deserializes a proof from JSON.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(json: &str) -> Result<WasmProof, JsValue> {
        Proof::from_json_slice(json.as_bytes())
            .map(WasmProof)
            .map_err(error_to_js)
    }

    /// Returns the proof format version.
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u16 {
        self.0.version().value()
    }

    /// Returns the epoch of the committee that certified this proof.
    #[wasm_bindgen(getter, js_name = checkpointEpoch)]
    pub fn checkpoint_epoch(&self) -> u64 {
        self.0.checkpoint_summary.epoch()
    }

    /// Verifies this proof locally with the supplied committee.
    pub fn verify(&self, committee: &WasmCommittee) -> Result<(), JsValue> {
        poi_rs::ProofVerifier::new(committee.inner())
            .verify(&self.0)
            .map_err(error_to_js)
    }

    /// Validates the proof format version.
    pub fn validate(&self) -> Result<(), JsValue> {
        self.0.validate().map_err(error_to_js)
    }

    /// Serializes this proof as JSON.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        let bytes = self.0.to_json_vec().map_err(error_to_js)?;
        String::from_utf8(bytes).map_err(error_to_js)
    }
}

/// Builds Proof of Inclusion evidence with an internal JavaScript ledger source.
#[wasm_bindgen(js_name = ProofBuilder)]
pub struct WasmProofBuilder(ProofBuilder<SourceAdapter>);

#[wasm_bindgen(js_class = ProofBuilder)]
impl WasmProofBuilder {
    /// Creates a builder backed by the provided JavaScript ledger source.
    #[wasm_bindgen(constructor)]
    pub fn new(source: LedgerSource) -> Self {
        Self(ProofBuilder::new(SourceAdapter::new(source)))
    }

    /// Adds a transaction target.
    pub fn transaction(self, transaction_digest: Uint8Array) -> Result<Self, JsValue> {
        let digest = TransactionDigest::from_bytes(transaction_digest.to_vec()).map_err(error_to_js)?;
        Ok(Self(self.0.transaction(digest)))
    }

    /// Adds an object target.
    pub fn object(self, object_id: Uint8Array) -> Result<Self, JsValue> {
        let object_id = ObjectId::from_bytes(object_id.to_vec()).map_err(error_to_js)?;
        Ok(Self(self.0.object(object_id)))
    }

    /// Adds an event target.
    pub fn event(self, transaction_digest: Uint8Array, event_sequence: u64) -> Result<Self, JsValue> {
        let tx_digest = TransactionDigest::from_bytes(transaction_digest.to_vec()).map_err(error_to_js)?;
        Ok(Self(self.0.event(EventID {
            tx_digest,
            event_seq: event_sequence,
        })))
    }

    /// Fetches the requested evidence and constructs the proof.
    pub async fn build(self) -> Result<WasmProof, JsValue> {
        self.0.build().await.map(WasmProof).map_err(error_to_js)
    }
}

pub(crate) fn error_to_js(error: impl Error) -> JsValue {
    let mut message = error.to_string();
    let mut source = error.source();

    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }

    js_sys::Error::new(&message).into()
}
