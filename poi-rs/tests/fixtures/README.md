# Proof Fixtures

The `v1` directory contains frozen fixtures for the version 1 proof format. The fixtures are synthetic test data, not captures from localnet or a public IOTA network. The transaction, object, and event fixtures use an epoch 0 single-validator test committee and share one generated checkpoint body.

`v1/consensus_commit_prologue.json` covers the JSON names that `iota-sdk-types` 1.0.0-rc.1 renamed. It was generated on the last release with the old names (commit `d2bfdbc`, iota v1.29.0): a consensus commit prologue with a canceled transaction, bound to the effects of a random test transaction with a failed status (`MoveAuthenticationError` wrapping `ExecutionCancelledDueToSharedObjectCongestionV2`) and a canceled unchanged shared object. It verifies against its own test committee in `v1/consensus_commit_prologue_committee.json`.

The `v2` directory contains the same proofs in the version 2 format: each `v1` fixture converted with `ProofV2::from` and serialized with `serde_json::to_string_pretty()`. They verify against the `v1` committees.

Do not rewrite the `v1` files for routine refactors. When the serialized proof format changes intentionally, generate a new versioned directory and update the compatibility tests to cover both versions as appropriate.

## Regeneration

Create a temporary Rust test beside `proof_serialization.rs` and construct the fixture data with the same test APIs used by `tests/utils/proofs.rs`:

- `Committee::new_simple_test_committee()` creates the committee and signing keys.
- `FullCheckpointContents::random_for_testing()` creates synthetic transaction data.
- `TestEffectsBuilder` adds the object or event effects required by each target.
- `CertifiedCheckpointSummary::new_from_keypairs_for_testing()` signs the checkpoint summary.
- `serde_json::to_string_pretty()` serializes the committee and proofs.

Write the committee once as `committee.json`, then serialize transaction-only, object and event target proofs as `transaction.json`, `object.json` and `event.json`.

Run the temporary generator with `cargo test --manifest-path poi-rs/Cargo.toml --test <generator-test> -- --test-threads=1`. The test helpers generate new random values, so compare the serialized structure rather than expecting identical bytes. Remove the generator after writing the files, inspect the fixture diff, and run `cargo test --manifest-path poi-rs/Cargo.toml --test proof_serialization -- --test-threads=1` before accepting a new fixture version.
