// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{error::Error, fmt};

use async_trait::async_trait;
use iota_sdk_types::{ObjectId, Version};
use iota_sdk_types::{SignedCheckpointSummary, SignedTransaction, Transaction, TransactionEffects, UserSignature};
use iota_types::{
    digests::TransactionDigest,
    effects::{TransactionEffectsAPI, TransactionEvents},
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContents},
};
use iota_types::{
    digests::{ChainIdentifier, CheckpointDigest},
    object::Object,
};
use js_sys::{Promise, Uint8Array};
use poi_rs::Source;
use poi_rs::{SourceCheckpoint, SourceError, SourceErrorKind, SourceTransaction};
use serde::Deserialize;
use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use wasm_bindgen_futures::JsFuture;

use crate::versioned::VersionedObject;
use crate::versioned::{VersionedCheckpointSummary, VersionedEvent, VersionedValidatorAggregatedSignature};

#[wasm_bindgen]
extern "C" {
    /// JavaScript source that owns the generated Node.js gRPC client.
    #[wasm_bindgen(typescript_type = "NodePoiSource")]
    pub type NodePoiSource;

    #[wasm_bindgen(method, catch, structural, js_name = chainIdentifier)]
    fn chain_identifier_js(this: &NodePoiSource) -> Result<Promise, JsValue>;

    #[wasm_bindgen(method, catch, structural, js_name = transaction)]
    fn transaction_js(this: &NodePoiSource, digest: Uint8Array) -> Result<Promise, JsValue>;

    #[wasm_bindgen(method, catch, structural, js_name = object)]
    fn object_js(this: &NodePoiSource, object_id: Uint8Array, version: Option<u64>) -> Result<Promise, JsValue>;

    #[wasm_bindgen(method, catch, structural, js_name = checkpoint)]
    fn checkpoint_js(this: &NodePoiSource, sequence_number: u64) -> Result<Promise, JsValue>;
}

pub(crate) struct WasmSource {
    source: NodePoiSource,
}

impl WasmSource {
    pub(crate) fn new(source: NodePoiSource) -> Self {
        Self { source }
    }

    async fn await_method(result: Result<Promise, JsValue>) -> Result<JsValue, BridgeError> {
        let promise = result.map_err(BridgeError::from_js)?;
        JsFuture::from(promise).await.map_err(BridgeError::from_js)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsTransactionEvidence {
    transaction_bcs: Vec<u8>,
    signatures_bcs: Vec<Vec<u8>>,
    effects_bcs: Vec<u8>,
    events_bcs: Option<Vec<Vec<u8>>>,
    checkpoint_sequence_number: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsCheckpointEvidence {
    summary_bcs: Vec<u8>,
    signature_bcs: Vec<u8>,
    contents_bcs: Vec<u8>,
}

#[async_trait(?Send)]
impl Source for WasmSource {
    async fn chain_identifier(&self, transaction_digest: TransactionDigest) -> Result<ChainIdentifier, SourceError> {
        let value = Self::await_method(self.source.chain_identifier_js())
            .await
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::FetchChainIdentifier {
                        source: Box::new(source),
                    },
                )
            })?;
        let bytes = Uint8Array::new(&value).to_vec();
        let digest = bytes.try_into().map_err(|bytes: Vec<u8>| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::ChainIdentifier {
                    source: Box::new(BridgeError(format!(
                        "chain identifier must contain 32 bytes, received {}",
                        bytes.len()
                    ))),
                },
            )
        })?;

        Ok(ChainIdentifier::from(CheckpointDigest::new(digest)))
    }

    async fn transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError> {
        let digest = Uint8Array::from(transaction_digest.as_ref());
        let value = Self::await_method(self.source.transaction_js(digest))
            .await
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::FetchTransaction {
                        source: Box::new(source),
                    },
                )
            })?;

        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }

        let evidence: JsTransactionEvidence = serde_wasm_bindgen::from_value(value).map_err(|source| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::Transaction {
                    source: Box::new(BridgeError(source.to_string())),
                },
            )
        })?;

        decode_transaction(transaction_digest, evidence).map(Some)
    }

    async fn object(&self, object_id: ObjectId, version: Option<Version>) -> Result<Option<Object>, SourceError> {
        let object_id_bytes = Uint8Array::from(object_id.as_ref());
        let value = Self::await_method(
            self.source
                .object_js(object_id_bytes, version.map(|version| version.as_u64())),
        )
        .await
        .map_err(|source| {
            SourceError::object(
                object_id,
                SourceErrorKind::FetchObject {
                    source: Box::new(source),
                },
            )
        })?;

        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }

        let bytes = Uint8Array::new(&value).to_vec();
        let versioned: VersionedObject = decode_bcs(&bytes).map_err(|source| {
            SourceError::object(
                object_id,
                SourceErrorKind::Object {
                    source: Box::new(source),
                },
            )
        })?;
        let VersionedObject::V1(object) = versioned;

        Ok(Some(object.into()))
    }

    async fn checkpoint(
        &self,
        transaction_digest: TransactionDigest,
        sequence_number: u64,
    ) -> Result<SourceCheckpoint, SourceError> {
        let value = Self::await_method(self.source.checkpoint_js(sequence_number))
            .await
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::FetchCheckpoint {
                        sequence_number,
                        source: Box::new(source),
                    },
                )
            })?;
        let evidence: JsCheckpointEvidence = serde_wasm_bindgen::from_value(value).map_err(|source| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::CheckpointSummary {
                    source: Box::new(BridgeError(source.to_string())),
                },
            )
        })?;

        decode_checkpoint(transaction_digest, evidence)
    }
}

fn decode_transaction(
    transaction_digest: TransactionDigest,
    evidence: JsTransactionEvidence,
) -> Result<SourceTransaction, SourceError> {
    let transaction: Transaction = decode_bcs(&evidence.transaction_bcs).map_err(|source| {
        SourceError::transaction(
            transaction_digest,
            SourceErrorKind::Transaction {
                source: Box::new(source),
            },
        )
    })?;
    let signatures = evidence
        .signatures_bcs
        .iter()
        .map(|bytes| decode_bcs::<UserSignature>(bytes))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::Signatures {
                    source: Box::new(source),
                },
            )
        })?;
    let transaction = SignedTransaction {
        transaction,
        signatures,
    }
    .try_into()
    .map_err(|source| {
        SourceError::transaction(
            transaction_digest,
            SourceErrorKind::Transaction {
                source: Box::new(source),
            },
        )
    })?;
    let effects: TransactionEffects = decode_bcs(&evidence.effects_bcs).map_err(|source| {
        SourceError::transaction(
            transaction_digest,
            SourceErrorKind::Effects {
                source: Box::new(source),
            },
        )
    })?;
    let events = if effects.events_digest().is_some() {
        let events_bcs = evidence.events_bcs.ok_or_else(|| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::MissingEvents {
                    source: Box::new(BridgeError(
                        "transaction effects commit to events but eventsBcs is missing".to_owned(),
                    )),
                },
            )
        })?;
        let events = events_bcs
            .iter()
            .map(|bytes| {
                let VersionedEvent::V1(event) = decode_bcs::<VersionedEvent>(bytes)?;
                Ok(event)
            })
            .collect::<Result<Vec<_>, bcs::Error>>()
            .map_err(|source| {
                SourceError::transaction(
                    transaction_digest,
                    SourceErrorKind::Events {
                        source: Box::new(source),
                    },
                )
            })?;

        Some(TransactionEvents(events))
    } else {
        None
    };

    Ok(SourceTransaction {
        transaction,
        effects,
        events,
        checkpoint_sequence_number: evidence.checkpoint_sequence_number,
    })
}

fn decode_checkpoint(
    transaction_digest: TransactionDigest,
    evidence: JsCheckpointEvidence,
) -> Result<SourceCheckpoint, SourceError> {
    let VersionedCheckpointSummary::V1(summary) = decode_bcs(&evidence.summary_bcs).map_err(|source| {
        SourceError::transaction(
            transaction_digest,
            SourceErrorKind::CheckpointSummary {
                source: Box::new(source),
            },
        )
    })?;
    let VersionedValidatorAggregatedSignature::V1(signature) =
        decode_bcs(&evidence.signature_bcs).map_err(|source| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::CheckpointSummary {
                    source: Box::new(source),
                },
            )
        })?;
    let summary: CertifiedCheckpointSummary = SignedCheckpointSummary {
        checkpoint: summary,
        signature,
    }
    .try_into()
    .map_err(|source| {
        SourceError::transaction(
            transaction_digest,
            SourceErrorKind::CheckpointSummary {
                source: Box::new(source),
            },
        )
    })?;
    let contents = decode_bcs::<iota_sdk_types::CheckpointContents>(&evidence.contents_bcs)
        .and_then(|contents| {
            CheckpointContents::try_from(contents).map_err(|source| bcs::Error::Custom(source.to_string()))
        })
        .map_err(|source| {
            SourceError::transaction(
                transaction_digest,
                SourceErrorKind::CheckpointContents {
                    source: Box::new(source),
                },
            )
        })?;

    Ok(SourceCheckpoint { summary, contents })
}

fn decode_bcs<T>(bytes: &[u8]) -> Result<T, bcs::Error>
where
    T: for<'de> Deserialize<'de>,
{
    bcs::from_bytes(bytes)
}

#[derive(Debug)]
struct BridgeError(String);

impl BridgeError {
    fn from_js(value: JsValue) -> Self {
        let message = value
            .dyn_ref::<js_sys::Error>()
            .map(js_sys::Error::message)
            .and_then(|message| message.as_string())
            .or_else(|| value.as_string())
            .unwrap_or_else(|| format!("{value:?}"));
        Self(message)
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for BridgeError {}

#[cfg(test)]
mod tests {
    use iota_sdk_types::{
        CheckpointContents as SdkCheckpointContents, SignedCheckpointSummary as SdkSignedCheckpointSummary,
        SignedTransaction as SdkSignedTransaction,
    };
    use poi_rs::Proof;

    use super::*;

    #[test]
    fn decodes_the_grpc_bcs_evidence_into_existing_iota_types() {
        let proof = Proof::from_json_slice(include_bytes!("../../../../poi-rs/tests/fixtures/v1/event.json"))
            .expect("fixture must deserialize");
        let transaction_digest = *proof.transaction_proof.transaction.digest();
        let signed_transaction: SdkSignedTransaction = proof
            .transaction_proof
            .transaction
            .clone()
            .try_into()
            .expect("transaction must convert to SDK types");
        let events_bcs = proof.transaction_proof.events.as_ref().map(|events| {
            events
                .0
                .iter()
                .cloned()
                .map(|event| bcs::to_bytes(&VersionedEvent::V1(event)).expect("event must serialize"))
                .collect()
        });
        let transaction = decode_transaction(
            transaction_digest,
            JsTransactionEvidence {
                transaction_bcs: bcs::to_bytes(&signed_transaction.transaction).expect("transaction must serialize"),
                signatures_bcs: signed_transaction
                    .signatures
                    .iter()
                    .map(|signature| bcs::to_bytes(signature).expect("signature must serialize"))
                    .collect(),
                effects_bcs: bcs::to_bytes(&proof.transaction_proof.effects).expect("effects must serialize"),
                events_bcs,
                checkpoint_sequence_number: proof.checkpoint_summary.sequence_number,
            },
        )
        .expect("transaction evidence must decode");

        assert_eq!(transaction.transaction, proof.transaction_proof.transaction);
        assert_eq!(transaction.effects, proof.transaction_proof.effects);
        assert_eq!(transaction.events, proof.transaction_proof.events);

        let signed_summary: SdkSignedCheckpointSummary = proof
            .checkpoint_summary
            .clone()
            .try_into()
            .expect("checkpoint summary must convert to SDK types");
        let contents = SdkCheckpointContents::try_from(proof.transaction_proof.checkpoint_contents.clone())
            .expect("checkpoint contents must convert to SDK types");
        let checkpoint = decode_checkpoint(
            transaction_digest,
            JsCheckpointEvidence {
                summary_bcs: bcs::to_bytes(&VersionedCheckpointSummary::V1(signed_summary.checkpoint))
                    .expect("checkpoint summary must serialize"),
                signature_bcs: bcs::to_bytes(&VersionedValidatorAggregatedSignature::V1(signed_summary.signature))
                    .expect("checkpoint signature must serialize"),
                contents_bcs: bcs::to_bytes(&contents).expect("checkpoint contents must serialize"),
            },
        )
        .expect("checkpoint evidence must decode");

        assert_eq!(
            bcs::to_bytes(&checkpoint.summary).expect("decoded checkpoint summary must serialize"),
            bcs::to_bytes(&proof.checkpoint_summary).expect("fixture checkpoint summary must serialize")
        );
        assert_eq!(checkpoint.contents, proof.transaction_proof.checkpoint_contents);
    }
}
