// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use iota_sdk_types::gas::GasCostSummary;
use iota_types::{
    base_types::{ExecutionData, ObjectRef},
    committee::Committee,
    digests::{ChainIdentifier, TransactionDigest},
    messages_checkpoint::{CertifiedCheckpointSummary, CheckpointContents, CheckpointSummary, FullCheckpointContents},
    object::Object,
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
            .filter(|object| object.compute_object_reference() == object_ref)
            .ok_or_else(|| SourceError::object(object_ref, SourceErrorKind::ObjectNotFound))?;
        let mut proof = self.transaction(object.previous_transaction).await?;
        proof.target = proof.target.add_object(object_ref, object);
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
    let object_ref = object.compute_object_reference();
    let source = MockSource {
        proof: Some(proof),
        object: Some(object.clone()),
    };

    let proof = source.object(object_ref).await.unwrap();

    assert_eq!(proof.transaction_proof.transaction.digest(), &transaction_digest);
    assert_eq!(proof.target.objects, vec![(object_ref, object)]);
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
    let object_ref = Object::immutable_for_testing().compute_object_reference();
    let source = MockSource::default();

    let result = source.object(object_ref).await;

    let error = result.unwrap_err();
    assert_eq!(error.target, SourceTarget::Object(object_ref));
    assert!(matches!(error.kind, SourceErrorKind::ObjectNotFound));
}
