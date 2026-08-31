# Proof Fixtures

The `v1` directory contains frozen fixtures for the version 1 proof format. The fixtures are synthetic test data, not captures from localnet or a public IOTA network. They use an epoch 0 single-validator test committee and share one generated checkpoint body.

Do not rewrite the `v1` files for routine refactors. When the serialized proof format changes intentionally, generate a new versioned directory and update the compatibility tests to cover both versions as appropriate.

## Regeneration

Create a temporary Rust test beside `proof_serialization.rs` and construct the fixture data with the same test APIs used by `tests/utils/proofs.rs`:

- `Committee::new_simple_test_committee()` creates the committee and signing keys.
- `FullCheckpointContents::random_for_testing()` creates synthetic transaction data.
- `TestEffectsBuilder` adds the object or event effects required by each target.
- `CertifiedCheckpointSummary::new_from_keypairs_for_testing()` signs the checkpoint summary.
- `serde_json::to_string_pretty()` serializes the committee and proofs.

Write the committee once as `committee.json`, then serialize transaction-only, object and event target proofs as `transaction.json`, `object.json` and `event.json`.

Run the temporary generator with `cargo test -p poi-rs --test <generator-test> -- --test-threads=1`. The test helpers generate new random values, so compare the serialized structure rather than expecting identical bytes. Remove the generator after writing the files, inspect the fixture diff, and run `cargo test -p poi-rs --test proof_serialization -- --test-threads=1` before accepting a new fixture version.
