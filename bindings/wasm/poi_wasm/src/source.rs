// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use async_trait::async_trait;
use fastcrypto::traits::ToFromBytes;
use iota_sdk_types::{
    CheckpointContents, CheckpointDigest, ObjectId, SignedCheckpointSummary, SignedTransaction, Transaction,
    TransactionDigest, UserSignature, Version,
};
use iota_types::{
    base_types::AuthorityName,
    committee::{Committee, EpochId},
    digests::ChainIdentifier,
    effects::{TransactionEffects, TransactionEffectsAPI, TransactionEvents},
    messages_checkpoint::CertifiedCheckpointSummary,
    object::Object,
};
use js_sys::Uint8Array;
use poi_rs::{Source, SourceCheckpoint, SourceError, SourceTransaction};
use serde::Deserialize;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::error::PoiError;
use crate::versioned::VersionedObject;
use crate::versioned::{VersionedCheckpointSummary, VersionedEvent, VersionedValidatorAggregatedSignature};

#[wasm_bindgen]
extern "C" {
    /// JavaScript source that owns the generated ledger client.
    #[derive(Clone)]
    #[wasm_bindgen(typescript_type = "LedgerSource")]
    pub type LedgerSource;

    #[wasm_bindgen(method, catch, structural, js_name = chainIdentifier)]
    async fn chain_identifier(this: &LedgerSource) -> Result<Uint8Array, JsValue>;

    #[wasm_bindgen(method, catch, structural)]
    async fn transaction(this: &LedgerSource, digest: Uint8Array) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, structural)]
    async fn object(this: &LedgerSource, object_id: Uint8Array, version: Option<u64>) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, structural)]
    async fn checkpoint(this: &LedgerSource, sequence_number: u64) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, structural)]
    async fn committee(this: &LedgerSource, epoch: u64) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, structural, js_name = currentEpoch)]
    async fn current_epoch(this: &LedgerSource) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method, catch, structural, js_name = epochCloseSummary)]
    async fn epoch_close_summary(this: &LedgerSource, epoch: u64) -> Result<JsValue, JsValue>;
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsCheckpointSummaryEvidence {
    summary_bcs: Vec<u8>,
    signature_bcs: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsCommittee {
    members: Vec<JsCommitteeMember>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsCommitteeMember {
    public_key: Vec<u8>,
    weight: u64,
}

#[async_trait(?Send)]
impl Source for LedgerSource {
    async fn chain_identifier(&self) -> Result<ChainIdentifier, SourceError> {
        let bytes = self
            .chain_identifier()
            .await
            .map_err(|source| SourceError::request(PoiError::from_js(source)))?
            .to_vec();
        let digest = bytes.try_into().map_err(|bytes: Vec<u8>| {
            SourceError::invalid_response(PoiError::invalid_response(format!(
                "chain identifier must contain 32 bytes, received {}",
                bytes.len()
            )))
        })?;

        Ok(ChainIdentifier::from(CheckpointDigest::new(digest)))
    }

    async fn transaction(
        &self,
        transaction_digest: TransactionDigest,
    ) -> Result<Option<SourceTransaction>, SourceError> {
        let digest = Uint8Array::from(transaction_digest.as_ref());
        let value = self
            .transaction(digest)
            .await
            .map_err(|source| SourceError::request(PoiError::from_js(source)))?;

        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }

        let evidence: JsTransactionEvidence = serde_wasm_bindgen::from_value(value)
            .map_err(|source| SourceError::invalid_response(PoiError::invalid_response(source.to_string())))?;

        decode_transaction(evidence).map(Some)
    }

    async fn object(&self, object_id: ObjectId, version: Option<Version>) -> Result<Option<Object>, SourceError> {
        let object_id_bytes = Uint8Array::from(object_id.as_ref());
        let value = self
            .object(object_id_bytes, version.map(|version| version.as_u64()))
            .await
            .map_err(|source| SourceError::request(PoiError::from_js(source)))?;

        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }

        let bytes = Uint8Array::new(&value).to_vec();
        let versioned: VersionedObject = decode_bcs(&bytes).map_err(SourceError::invalid_response)?;
        let VersionedObject::V1(object) = versioned;

        Ok(Some(object.into()))
    }

    async fn checkpoint(&self, sequence_number: u64) -> Result<SourceCheckpoint, SourceError> {
        let value = self
            .checkpoint(sequence_number)
            .await
            .map_err(|source| SourceError::request(PoiError::from_js(source)))?;
        let evidence: JsCheckpointEvidence = serde_wasm_bindgen::from_value(value)
            .map_err(|source| SourceError::invalid_response(PoiError::invalid_response(source.to_string())))?;

        decode_checkpoint(evidence)
    }

    async fn committee(&self, epoch: EpochId) -> Result<Committee, SourceError> {
        let value = self
            .committee(epoch)
            .await
            .map_err(|source| SourceError::request(PoiError::from_js(source)))?;
        let evidence: JsCommittee = serde_wasm_bindgen::from_value(value)
            .map_err(|source| SourceError::invalid_response(PoiError::invalid_response(source.to_string())))?;

        decode_committee(epoch, evidence).map_err(SourceError::invalid_response)
    }

    async fn current_epoch(&self) -> Result<Option<EpochId>, SourceError> {
        let value = self
            .current_epoch()
            .await
            .map_err(|source| SourceError::request(PoiError::from_js(source)))?;

        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }

        let epoch = serde_wasm_bindgen::from_value(value)
            .map_err(|source| SourceError::invalid_response(PoiError::invalid_response(source.to_string())))?;

        Ok(Some(epoch))
    }

    async fn epoch_close_summary(&self, epoch: EpochId) -> Result<Option<CertifiedCheckpointSummary>, SourceError> {
        let value = self
            .epoch_close_summary(epoch)
            .await
            .map_err(|source| SourceError::request(PoiError::from_js(source)))?;

        if value.is_undefined() || value.is_null() {
            return Ok(None);
        }

        let evidence: JsCheckpointSummaryEvidence = serde_wasm_bindgen::from_value(value)
            .map_err(|source| SourceError::invalid_response(PoiError::invalid_response(source.to_string())))?;

        decode_certified_summary(&evidence.summary_bcs, &evidence.signature_bcs)
            .map(Some)
            .map_err(SourceError::invalid_response)
    }
}

fn decode_transaction(evidence: JsTransactionEvidence) -> Result<SourceTransaction, SourceError> {
    let transaction: Transaction = decode_bcs(&evidence.transaction_bcs).map_err(SourceError::invalid_response)?;
    let signatures = evidence
        .signatures_bcs
        .iter()
        .map(|bytes| decode_bcs::<UserSignature>(bytes))
        .collect::<Result<Vec<_>, _>>()
        .map_err(SourceError::invalid_response)?;
    let transaction: iota_types::transaction::Transaction = SignedTransaction {
        transaction,
        signatures,
    }
    .into();
    let effects: TransactionEffects = decode_bcs(&evidence.effects_bcs).map_err(SourceError::invalid_response)?;
    let events = if effects.events_digest().is_some() {
        let events_bcs = evidence.events_bcs.ok_or_else(|| {
            SourceError::missing_data(PoiError::invalid_response(
                "transaction effects commit to events but eventsBcs is missing",
            ))
        })?;
        let events = events_bcs
            .iter()
            .map(|bytes| {
                let VersionedEvent::V1(event) = decode_bcs::<VersionedEvent>(bytes)?;
                Ok(event)
            })
            .collect::<Result<Vec<_>, bcs::Error>>()
            .map_err(SourceError::invalid_response)?;

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

fn decode_checkpoint(evidence: JsCheckpointEvidence) -> Result<SourceCheckpoint, SourceError> {
    let summary = decode_certified_summary(&evidence.summary_bcs, &evidence.signature_bcs)
        .map_err(SourceError::invalid_response)?;
    let contents: CheckpointContents = decode_bcs(&evidence.contents_bcs).map_err(SourceError::invalid_response)?;

    Ok(SourceCheckpoint { summary, contents })
}

fn decode_certified_summary(summary_bcs: &[u8], signature_bcs: &[u8]) -> Result<CertifiedCheckpointSummary, PoiError> {
    let VersionedCheckpointSummary::V1(summary) =
        decode_bcs(summary_bcs).map_err(|source| PoiError::invalid_response(source.to_string()))?;
    let VersionedValidatorAggregatedSignature::V1(signature) =
        decode_bcs(signature_bcs).map_err(|source| PoiError::invalid_response(source.to_string()))?;

    SignedCheckpointSummary {
        checkpoint: summary,
        signature,
    }
    .try_into()
    .map_err(
        |source: iota_types::iota_sdk_types_conversions::SdkTypeConversionError| {
            PoiError::invalid_response(source.to_string())
        },
    )
}

fn decode_committee(epoch: EpochId, evidence: JsCommittee) -> Result<Committee, PoiError> {
    let voting_rights = evidence
        .members
        .into_iter()
        .map(|member| {
            AuthorityName::from_bytes(&member.public_key)
                .map(|authority| (authority, member.weight))
                .map_err(|source| PoiError::invalid_response(format!("invalid committee public key: {source}")))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    Ok(Committee::new(epoch, voting_rights))
}

fn decode_bcs<T>(bytes: &[u8]) -> Result<T, bcs::Error>
where
    T: for<'de> Deserialize<'de>,
{
    bcs::from_bytes(bytes)
}

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
        let proof = Proof::from_json_slice(include_bytes!("../../../../poi-rs/tests/fixtures/current/event.json"))
            .expect("fixture must deserialize");
        let transaction_proof = proof.transaction_proof();
        let checkpoint_summary = proof.checkpoint_summary();
        let checkpoint_contents = proof.checkpoint_contents();
        let signed_transaction: SdkSignedTransaction = transaction_proof
            .transaction
            .clone()
            .try_into()
            .expect("transaction must convert to SDK types");
        let events_bcs = transaction_proof.events.as_ref().map(|events| {
            events
                .0
                .iter()
                .cloned()
                .map(|event| bcs::to_bytes(&VersionedEvent::V1(event)).expect("event must serialize"))
                .collect()
        });
        let transaction = decode_transaction(JsTransactionEvidence {
            transaction_bcs: bcs::to_bytes(&signed_transaction.transaction).expect("transaction must serialize"),
            signatures_bcs: signed_transaction
                .signatures
                .iter()
                .map(|signature| bcs::to_bytes(signature).expect("signature must serialize"))
                .collect(),
            effects_bcs: bcs::to_bytes(&transaction_proof.effects).expect("effects must serialize"),
            events_bcs,
            checkpoint_sequence_number: checkpoint_summary.sequence_number,
        })
        .expect("transaction evidence must decode");

        assert_eq!(transaction.transaction, transaction_proof.transaction);
        assert_eq!(transaction.effects, transaction_proof.effects);
        assert_eq!(transaction.events, transaction_proof.events);

        let signed_summary: SdkSignedCheckpointSummary = checkpoint_summary
            .clone()
            .try_into()
            .expect("checkpoint summary must convert to SDK types");
        let contents = SdkCheckpointContents::try_from(checkpoint_contents.clone())
            .expect("checkpoint contents must convert to SDK types");
        let checkpoint = decode_checkpoint(JsCheckpointEvidence {
            summary_bcs: bcs::to_bytes(&VersionedCheckpointSummary::V1(signed_summary.checkpoint))
                .expect("checkpoint summary must serialize"),
            signature_bcs: bcs::to_bytes(&VersionedValidatorAggregatedSignature::V1(signed_summary.signature))
                .expect("checkpoint signature must serialize"),
            contents_bcs: bcs::to_bytes(&contents).expect("checkpoint contents must serialize"),
        })
        .expect("checkpoint evidence must decode");

        assert_eq!(
            bcs::to_bytes(&checkpoint.summary).expect("decoded checkpoint summary must serialize"),
            bcs::to_bytes(checkpoint_summary).expect("fixture checkpoint summary must serialize")
        );
        assert_eq!(&checkpoint.contents, checkpoint_contents);
    }
}
