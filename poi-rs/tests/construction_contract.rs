// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_sdk_types::{Event, gas::GasCostSummary};
use iota_types::{
    base_types::{ExecutionData, ObjectRef},
    committee::Committee,
    digests::{ChainIdentifier, TransactionDigest},
    effects::TransactionEvents,
    event::EventID,
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContents, CheckpointSummary, FullCheckpointContents},
    object::Object,
    sdk_types::{Address, Identifier, ObjectId, StructTag},
};
use poi_rs::{
    Proof, ProofTargets, ProofVerifier, Source, SourceError, SourceErrorKind, SourceTarget, TransactionProof,
};

#[derive(Default)]
struct MockSource {
    proof: Option<Proof>,
    object: Option<Object>,
}

#[async_trait]
impl Source for MockSource {
    async fn transaction(&self, transaction_digest: TransactionDigest) -> Result<Proof, SourceError> {
        self.proof
            .clone()
            .ok_or_else(|| SourceError::new(transaction_digest, SourceErrorKind::TransactionNotFound))
    }

    async fn object(&self, object_ref: ObjectRef) -> Result<Proof, SourceError> {
        let object = self
            .object
            .clone()
            .filter(|object| object.as_inner().object_ref() == object_ref)
            .ok_or_else(|| SourceError::object(object_ref, SourceErrorKind::ObjectNotFound))?;
        let mut proof = self.transaction(object.previous_transaction).await?;
        proof.target = proof.target.add_object(object_ref, object);
        Ok(proof)
    }

    async fn event(&self, event_id: EventID) -> Result<Proof, SourceError> {
        let mut proof = self.transaction(event_id.tx_digest).await?;
        let event = proof
            .transaction_proof
            .events
            .as_ref()
            .and_then(|events| events.get(event_id.event_seq as usize))
            .cloned()
            .ok_or_else(|| SourceError::event(event_id, SourceErrorKind::EventNotFound))?;
        proof.target = proof.target.add_event(event_id, event);
        Ok(proof)
    }
}

fn test_execution_data() -> ExecutionData {
    FullCheckpointContents::random_for_testing()
        .into_iter()
        .next()
        .expect("test checkpoint contents includes one transaction")
}

fn test_proof() -> (Committee, TransactionDigest, Proof) {
    let execution_data = test_execution_data();
    let transaction_digest = *execution_data.transaction.digest();
    let checkpoint_contents = CheckpointContents::new_with_digests_only_for_tests([execution_data.digests()]);
    let checkpoint_summary = CheckpointSummary {
        epoch: 0,
        sequence_number: 0,
        network_total_transactions: checkpoint_contents.size() as u64,
        content_digest: *checkpoint_contents.digest(),
        previous_digest: None,
        epoch_rolling_gas_cost_summary: GasCostSummary::default(),
        timestamp_ms: 0,
        checkpoint_commitments: Vec::new(),
        end_of_epoch_data: None,
        version_specific_data: Vec::new(),
    };
    let (committee, keypairs) = Committee::new_simple_test_committee();
    let checkpoint_summary =
        CertifiedCheckpointSummary::new_from_keypairs_for_testing(checkpoint_summary, &keypairs, &committee);
    let chain = ChainIdentifier::from(*checkpoint_summary.digest());
    let proof = Proof::new(
        chain,
        ProofTargets::new(),
        checkpoint_summary,
        TransactionProof::new(
            checkpoint_contents,
            execution_data.transaction,
            execution_data.effects,
            None,
        ),
    );

    (committee, transaction_digest, proof)
}

fn test_event(contents: Vec<u8>) -> Event {
    Event {
        package_id: ObjectId::SYSTEM,
        module: Identifier::IOTA_SYSTEM_MODULE,
        sender: Address::SYSTEM,
        type_: StructTag::new(
            Address::SYSTEM,
            Identifier::IOTA_SYSTEM_MODULE,
            Identifier::SYSTEM_EPOCH_INFO_EVENT,
            Vec::new(),
        ),
        contents,
    }
}

#[tokio::test]
async fn source_builds_transaction_proof() {
    let (committee, transaction_digest, proof) = test_proof();
    let source = MockSource {
        proof: Some(proof),
        object: None,
    };

    let proof = source.transaction(transaction_digest).await.unwrap();

    assert_eq!(proof.transaction_proof.transaction.digest(), &transaction_digest);
    ProofVerifier::new(&committee).verify(&proof).unwrap();
}

#[tokio::test]
async fn source_builds_object_proof() {
    let (_, transaction_digest, proof) = test_proof();
    let mut object = Object::immutable_for_testing();
    object.previous_transaction = transaction_digest;
    let object_ref = object.as_inner().object_ref();
    let source = MockSource {
        proof: Some(proof),
        object: Some(object.clone()),
    };

    let proof = source.object(object_ref).await.unwrap();

    assert_eq!(proof.transaction_proof.transaction.digest(), &transaction_digest);
    assert_eq!(proof.target.objects, vec![(object_ref, object)]);
}

#[tokio::test]
async fn source_builds_event_proof() {
    let (_, transaction_digest, mut proof) = test_proof();
    let event = test_event(vec![1, 2, 3]);
    proof.transaction_proof.events = Some(TransactionEvents(vec![event.clone()]));
    let event_id = EventID {
        tx_digest: transaction_digest,
        event_seq: 0,
    };
    let source = MockSource {
        proof: Some(proof),
        object: None,
    };

    let proof = source.event(event_id).await.unwrap();

    assert_eq!(proof.transaction_proof.transaction.digest(), &transaction_digest);
    assert_eq!(proof.target.events, vec![(event_id, event)]);
}

#[tokio::test]
async fn transaction_surfaces_source_failures() {
    let (_, transaction_digest, _) = test_proof();
    let source = MockSource::default();

    let result = source.transaction(transaction_digest).await;

    let error = result.unwrap_err();
    assert_eq!(error.target, SourceTarget::Transaction(transaction_digest));
    assert!(matches!(error.kind, SourceErrorKind::TransactionNotFound));
}

#[tokio::test]
async fn object_surfaces_source_failures() {
    let object_ref = Object::immutable_for_testing().as_inner().object_ref();
    let source = MockSource::default();

    let result = source.object(object_ref).await;

    let error = result.unwrap_err();
    assert_eq!(error.target, SourceTarget::Object(object_ref));
    assert!(matches!(error.kind, SourceErrorKind::ObjectNotFound));
}

#[tokio::test]
async fn event_surfaces_source_failures() {
    let (_, transaction_digest, proof) = test_proof();
    let event_id = EventID {
        tx_digest: transaction_digest,
        event_seq: 0,
    };
    let source = MockSource {
        proof: Some(proof),
        object: None,
    };

    let result = source.event(event_id).await;

    let error = result.unwrap_err();
    assert_eq!(error.target, SourceTarget::Event(event_id));
    assert!(matches!(error.kind, SourceErrorKind::EventNotFound));
}
