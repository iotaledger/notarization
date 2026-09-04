// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk_types::{ObjectId, TransactionDigest};
use iota_types::event::EventID;
use js_sys::Uint8Array;
use poi_rs::{Proof, ProofBuilder, ProofTargets, VerifiedProof};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::committee::WasmCommittee;
use crate::error::{PoiError, WasmResult};
use crate::source::LedgerSource;

/// An object selected as a Proof of Inclusion target.
#[wasm_bindgen(js_name = ProofObjectTarget, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmProofObjectTarget {
    /// Object ID selected by the caller.
    #[wasm_bindgen(js_name = objectId)]
    pub object_id: String,
    /// Exact object version resolved by the proof builder.
    pub version: u64,
    /// Digest committing to the selected object version.
    pub digest: String,
}

/// An event selected as a Proof of Inclusion target.
#[wasm_bindgen(js_name = ProofEventTarget, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmProofEventTarget {
    /// Digest of the transaction that emitted the event.
    #[wasm_bindgen(js_name = transactionDigest)]
    pub transaction_digest: String,
    /// Transaction-local event sequence number.
    #[wasm_bindgen(js_name = eventSequence)]
    pub event_sequence: u64,
}

/// Values explicitly selected for a Proof of Inclusion proof.
#[wasm_bindgen(js_name = ProofTargets, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmProofTargets {
    /// Transaction explicitly selected by the caller, when present.
    pub transaction: Option<String>,
    /// Exact object versions selected by the caller and resolved by the builder.
    pub objects: Vec<WasmProofObjectTarget>,
    /// Events explicitly selected by the caller.
    pub events: Vec<WasmProofEventTarget>,
}

impl From<&ProofTargets> for WasmProofTargets {
    fn from(value: &ProofTargets) -> Self {
        let objects = value
            .objects
            .iter()
            .map(|object| {
                let object_ref = object.as_inner().object_ref();
                WasmProofObjectTarget {
                    object_id: object_ref.object_id.to_string(),
                    version: object_ref.version.as_u64(),
                    digest: object_ref.digest.to_string(),
                }
            })
            .collect();
        let events = value
            .events
            .iter()
            .map(|event| WasmProofEventTarget {
                transaction_digest: event.tx_digest.to_string(),
                event_sequence: event.event_seq,
            })
            .collect();

        Self {
            transaction: value.transaction.map(|transaction| transaction.to_string()),
            objects,
            events,
        }
    }
}

/// Authenticated targets returned by successful proof verification.
#[wasm_bindgen(js_name = VerifiedProof, inspectable)]
#[derive(Clone)]
pub struct WasmVerifiedProof(Proof);

impl WasmVerifiedProof {
    pub(crate) fn new<'proof>(proof: &'proof Proof, _verified: VerifiedProof<'proof>) -> Self {
        Self(proof.clone())
    }
}

#[wasm_bindgen(js_class = VerifiedProof)]
impl WasmVerifiedProof {
    /// Returns the epoch of the authenticated checkpoint.
    #[wasm_bindgen(getter, js_name = checkpointEpoch)]
    pub fn checkpoint_epoch(&self) -> u64 {
        self.0.checkpoint_summary().epoch()
    }

    /// Returns the authenticated checkpoint sequence number.
    #[wasm_bindgen(getter, js_name = checkpointSequenceNumber)]
    pub fn checkpoint_sequence_number(&self) -> u64 {
        self.0.checkpoint_summary().sequence_number
    }

    /// Returns the authenticated checkpoint timestamp in milliseconds since the Unix epoch.
    #[wasm_bindgen(getter, js_name = checkpointTimestampMs)]
    pub fn checkpoint_timestamp_ms(&self) -> u64 {
        self.0.checkpoint_summary().timestamp_ms
    }

    /// Returns the digest of the transaction included in the authenticated checkpoint.
    #[wasm_bindgen(getter)]
    pub fn transaction(&self) -> String {
        self.0.transaction_proof().transaction.digest().to_string()
    }

    /// Returns the authenticated transaction, object, and event targets.
    #[wasm_bindgen(getter)]
    pub fn targets(&self) -> WasmProofTargets {
        self.0.targets().into()
    }

    /// Returns a selected authenticated object encoded as BCS.
    #[wasm_bindgen(js_name = objectBcs)]
    pub fn object_bcs(&self, target_index: u32) -> WasmResult<Uint8Array> {
        let object =
            self.0.targets().objects.get(target_index as usize).ok_or_else(|| {
                PoiError::invalid_input(format!("object target index {target_index} is out of bounds"))
            })?;
        let bytes = bcs::to_bytes(object)?;

        Ok(Uint8Array::from(bytes.as_slice()))
    }

    /// Returns the contents of a selected authenticated event.
    #[wasm_bindgen(js_name = eventContents)]
    pub fn event_contents(&self, target_index: u32) -> WasmResult<Uint8Array> {
        let event_id =
            self.0.targets().events.get(target_index as usize).ok_or_else(|| {
                PoiError::invalid_input(format!("event target index {target_index} is out of bounds"))
            })?;
        let events = self
            .0
            .transaction_proof()
            .events
            .as_ref()
            .ok_or_else(|| PoiError::invalid_response("verified proof is missing event data"))?;
        let event_index = usize::try_from(event_id.event_seq)
            .map_err(|_| PoiError::invalid_response("verified event sequence exceeds the platform index range"))?;
        let event = events
            .0
            .get(event_index)
            .ok_or_else(|| PoiError::invalid_response("verified event sequence is out of bounds"))?;

        Ok(Uint8Array::from(event.contents.as_slice()))
    }
}

/// Proof of Inclusion evidence constructed by `poi-rs`.
#[wasm_bindgen(js_name = Proof)]
pub struct WasmProof(pub(crate) Proof);

#[wasm_bindgen(js_class = Proof)]
impl WasmProof {
    /// Deserializes a proof from JSON.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(json: &str) -> WasmResult<WasmProof> {
        let proof = Proof::from_json_slice(json.as_bytes())?;

        Ok(WasmProof(proof))
    }

    /// Returns the proof format version.
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u16 {
        self.0.version()
    }

    /// Returns the epoch of the committee that certified this proof.
    #[wasm_bindgen(getter, js_name = checkpointEpoch)]
    pub fn checkpoint_epoch(&self) -> u64 {
        self.0.checkpoint_summary().epoch()
    }

    /// Returns the transaction, object, and event targets selected for this proof.
    #[wasm_bindgen(getter)]
    pub fn targets(&self) -> WasmProofTargets {
        self.0.targets().into()
    }

    /// Verifies this proof locally and returns its authenticated targets.
    pub fn verify(&self, committee: &WasmCommittee) -> WasmResult<WasmVerifiedProof> {
        let verified = poi_rs::ProofVerifier::new(committee.inner()).verify(&self.0)?;

        Ok(WasmVerifiedProof::new(&self.0, verified))
    }

    /// Serializes this proof as JSON.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmResult<String> {
        let bytes = self.0.to_json_vec()?;

        Ok(String::from_utf8(bytes)?)
    }
}

/// Builds Proof of Inclusion evidence with an internal JavaScript ledger source.
#[wasm_bindgen(js_name = ProofBuilder)]
pub struct WasmProofBuilder(ProofBuilder<LedgerSource>);

#[wasm_bindgen(js_class = ProofBuilder)]
impl WasmProofBuilder {
    /// Creates a builder backed by the provided JavaScript ledger source.
    #[wasm_bindgen(constructor)]
    pub fn new(source: LedgerSource) -> Self {
        Self(ProofBuilder::new(source))
    }

    /// Adds a transaction proof target.
    pub fn transaction(self, transaction_digest: Uint8Array) -> WasmResult<Self> {
        let digest = TransactionDigest::from_bytes(transaction_digest.to_vec())?;
        Ok(Self(self.0.transaction(digest)))
    }

    /// Adds an object proof target.
    ///
    /// Without a transaction or event target, the source resolves the object's
    /// latest version at proof construction time.
    pub fn object(self, object_id: Uint8Array) -> WasmResult<Self> {
        let object_id = ObjectId::from_bytes(object_id.to_vec())?;
        Ok(Self(self.0.object(object_id)))
    }

    /// Adds an event proof target.
    pub fn event(self, transaction_digest: Uint8Array, event_sequence: u64) -> WasmResult<Self> {
        let tx_digest = TransactionDigest::from_bytes(transaction_digest.to_vec())?;
        Ok(Self(self.0.event(EventID {
            tx_digest,
            event_seq: event_sequence,
        })))
    }

    /// Fetches the requested evidence and constructs the proof.
    pub async fn build(self) -> WasmResult<WasmProof> {
        let proof = self.0.build().await?;

        Ok(WasmProof(proof))
    }
}
