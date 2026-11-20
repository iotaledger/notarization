// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! JSON compatibility for [`ProofV1`](super::ProofV1).
//!
//! Version 1 proofs were written with `iota-sdk-types` before 1.0.0-rc.1, which spelled six identifiers
//! differently from the current types (`Cancelled` instead of `Canceled`, `MoveAuthenticationError` instead
//! of `MoveAuthentication`). The BCS encoding of the affected types did not change, so the difference is
//! limited to these JSON names. [`rename`] converts the body of a version 1 proof between its original
//! spelling and the spelling of the current types, at the only places in a proof where the affected types
//! occur: the consensus commit prologue of the transaction, and the execution status and the unchanged
//! shared objects of the effects.

use serde_json::{Map, Value};

/// `ConsensusDeterminedVersionAssignments` variant, as (version 1 spelling, current spelling).
const CANCELED_TRANSACTIONS: (&str, &str) = ("CancelledTransactions", "CanceledTransactions");
/// Field of that variant.
const CANCELED_TRANSACTIONS_FIELD: (&str, &str) = ("cancelled_transactions", "canceled_transactions");
/// `UnchangedSharedKind` variant.
const CANCELED_SHARED_OBJECT: (&str, &str) = ("Cancelled", "Canceled");
/// `ExecutionError` variant that nests another `ExecutionError` in its `error` field.
const MOVE_AUTHENTICATION: (&str, &str) = ("MoveAuthenticationError", "MoveAuthentication");
/// All renamed `ExecutionError` variants.
const EXECUTION_ERRORS: [(&str, &str); 4] = [
    (
        "ExecutionCancelledDueToSharedObjectCongestion",
        "ExecutionCanceledDueToSharedObjectCongestion",
    ),
    (
        "ExecutionCancelledDueToRandomnessUnavailable",
        "ExecutionCanceledDueToRandomnessUnavailable",
    ),
    (
        "ExecutionCancelledDueToSharedObjectCongestionV2",
        "ExecutionCanceledDueToSharedObjectCongestionV2",
    ),
    MOVE_AUTHENTICATION,
];

/// The direction of a conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    /// From the version 1 spelling to the spelling of the current types.
    FromV1,
    /// From the spelling of the current types to the version 1 spelling.
    ToV1,
}

impl Direction {
    /// Orders a (version 1, current) identifier pair as (from, to).
    const fn order<'a>(self, (v1, current): (&'a str, &'a str)) -> (&'a str, &'a str) {
        match self {
            Self::FromV1 => (v1, current),
            Self::ToV1 => (current, v1),
        }
    }
}

/// Renames the identifiers in the body of a version 1 proof, the value of its `ProofV1` key.
pub(super) fn rename(proof: &mut Value, direction: Direction) {
    let Some(transaction_proof) = proof.get_mut("transaction_proof") else {
        return;
    };
    if let Some(transaction) = transaction_proof.get_mut("transaction") {
        rename_version_assignments(transaction, direction);
    }
    // Effects are tagged by their version, for example `{ "V1": { ... } }`.
    if let Some(Value::Object(effects)) = transaction_proof.get_mut("effects") {
        for effects in effects.values_mut() {
            if let Some(error) = effects.pointer_mut("/status/status/error") {
                rename_execution_error(error, direction);
            }
            if let Some(Value::Array(objects)) = effects.get_mut("unchanged_shared_objects") {
                for kind in objects.iter_mut().filter_map(|object| object.get_mut("kind")) {
                    rename_key(kind, direction.order(CANCELED_SHARED_OBJECT));
                }
            }
        }
    }
}

/// Renames the canceled-transactions variant of the consensus-determined version assignments.
fn rename_version_assignments(value: &mut Value, direction: Direction) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if key == "consensus_determined_version_assignments" {
                    if let Some(assignments) = rename_key(child, direction.order(CANCELED_TRANSACTIONS)) {
                        rename_key(assignments, direction.order(CANCELED_TRANSACTIONS_FIELD));
                    }
                } else {
                    rename_version_assignments(child, direction);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                rename_version_assignments(item, direction);
            }
        }
        _ => {}
    }
}

/// Renames an execution error and the error nested in a Move authentication failure.
fn rename_execution_error(error: &mut Value, direction: Direction) {
    // Variants without fields serialize as a string, variants with fields as a single-key object.
    let name = match error {
        Value::String(name) => name.clone(),
        Value::Object(map) if map.len() == 1 => map.keys().next().cloned().unwrap_or_default(),
        _ => return,
    };
    let renamed = EXECUTION_ERRORS
        .iter()
        .map(|&pair| direction.order(pair))
        .find(|(from, _)| *from == name)
        .map(|(_, to)| to);

    match error {
        Value::String(name) => {
            if let Some(to) = renamed {
                *name = to.to_owned();
            }
        }
        _ => {
            let key = renamed.unwrap_or(&name);
            let fields = match renamed {
                Some(to) => rename_key(error, (&name, to)),
                None => error.get_mut(key),
            };
            if key == direction.order(MOVE_AUTHENTICATION).1 {
                if let Some(nested) = fields.and_then(|fields| fields.get_mut("error")) {
                    rename_execution_error(nested, direction);
                }
            }
        }
    }
}

/// Renames the key `from` of an object to `to`, keeping the key order, and returns its value.
fn rename_key<'value>(value: &'value mut Value, (from, to): (&str, &str)) -> Option<&'value mut Value> {
    let Value::Object(map) = value else {
        return None;
    };
    if !map.contains_key(from) {
        return None;
    }
    let renamed: Map<String, Value> = std::mem::take(map)
        .into_iter()
        .map(|(key, child)| {
            if key == from {
                (to.to_owned(), child)
            } else {
                (key, child)
            }
        })
        .collect();
    *map = renamed;
    map.get_mut(to)
}

#[cfg(test)]
mod tests {
    use iota_sdk_types::{ConsensusDeterminedVersionAssignments, ExecutionStatus, UnchangedSharedObject};
    use serde::Serialize;
    use serde::de::DeserializeOwned;
    use serde_json::json;

    use super::*;

    // Test vectors: each value serialized by iota-sdk-types at git revision 2f021d0 (the types of
    // version 1 proofs) and by iota-sdk-types 1.0.0-rc.2.
    const ASSIGNMENTS_V1: &str = r#"{"CancelledTransactions":{"cancelled_transactions":[{"digest":"8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR","version_assignments":[{"object_id":"0x0101010101010101010101010101010101010101010101010101010101010101","version":"3"}]}]}}"#;
    const ASSIGNMENTS_V2: &str = r#"{"CanceledTransactions":{"canceled_transactions":[{"digest":"8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR","version_assignments":[{"object_id":"0x0101010101010101010101010101010101010101010101010101010101010101","version":"3"}]}]}}"#;
    const UNCHANGED_V1: &str = r#"{"object_id":"0x0101010101010101010101010101010101010101010101010101010101010101","kind":{"Cancelled":{"version":"5"}}}"#;
    const UNCHANGED_V2: &str = r#"{"object_id":"0x0101010101010101010101010101010101010101010101010101010101010101","kind":{"Canceled":{"version":"5"}}}"#;
    const STATUS: [(&str, &str); 5] = [
        (
            r#"{"success":false,"status":{"error":{"ExecutionCancelledDueToSharedObjectCongestion":{"congested_objects":["0x0101010101010101010101010101010101010101010101010101010101010101"]}},"command":1}}"#,
            r#"{"success":false,"status":{"error":{"ExecutionCanceledDueToSharedObjectCongestion":{"congested_objects":["0x0101010101010101010101010101010101010101010101010101010101010101"]}},"command":1}}"#,
        ),
        (
            r#"{"success":false,"status":{"error":"ExecutionCancelledDueToRandomnessUnavailable","command":1}}"#,
            r#"{"success":false,"status":{"error":"ExecutionCanceledDueToRandomnessUnavailable","command":1}}"#,
        ),
        (
            r#"{"success":false,"status":{"error":{"ExecutionCancelledDueToSharedObjectCongestionV2":{"congested_objects":["0x0101010101010101010101010101010101010101010101010101010101010101"],"suggested_gas_price":"7"}},"command":1}}"#,
            r#"{"success":false,"status":{"error":{"ExecutionCanceledDueToSharedObjectCongestionV2":{"congested_objects":["0x0101010101010101010101010101010101010101010101010101010101010101"],"suggested_gas_price":"7"}},"command":1}}"#,
        ),
        (
            r#"{"success":false,"status":{"error":{"MoveAuthenticationError":{"error":"InsufficientGas"}},"command":1}}"#,
            r#"{"success":false,"status":{"error":{"MoveAuthentication":{"error":"InsufficientGas"}},"command":1}}"#,
        ),
        (
            r#"{"success":false,"status":{"error":"InsufficientGas","command":1}}"#,
            r#"{"success":false,"status":{"error":"InsufficientGas","command":1}}"#,
        ),
    ];

    /// A proof body with the affected values at the places they occur in a real proof.
    fn proof_body(assignments: &str, status: &str, unchanged: &str) -> Value {
        json!({
            "targets": { "transaction": null, "objects": [], "events": [] },
            "transaction_proof": {
                "transaction": { "data": { "transaction": { "V1": { "kind": { "ConsensusCommitPrologueV1": {
                    "consensus_determined_version_assignments": serde_json::from_str::<Value>(assignments).unwrap()
                } } } } } },
                "effects": { "V1": {
                    "status": serde_json::from_str::<Value>(status).unwrap(),
                    "unchanged_shared_objects": [serde_json::from_str::<Value>(unchanged).unwrap()]
                } },
                "events": null
            }
        })
    }

    /// Deserializes `json` into the current type and serializes it again.
    fn current_round_trip<T: DeserializeOwned + Serialize>(json: &str) -> String {
        serde_json::to_string(&serde_json::from_str::<T>(json).expect("must match the current types")).unwrap()
    }

    #[test]
    fn test_vectors_match_the_current_types() {
        assert_eq!(
            current_round_trip::<ConsensusDeterminedVersionAssignments>(ASSIGNMENTS_V2),
            ASSIGNMENTS_V2
        );
        assert_eq!(current_round_trip::<UnchangedSharedObject>(UNCHANGED_V2), UNCHANGED_V2);
        for (_, current) in STATUS {
            assert_eq!(current_round_trip::<ExecutionStatus>(current), current);
        }
    }

    #[test]
    fn renames_every_affected_identifier_in_both_directions() {
        for (v1_status, current_status) in STATUS {
            let v1 = proof_body(ASSIGNMENTS_V1, v1_status, UNCHANGED_V1);
            let current = proof_body(ASSIGNMENTS_V2, current_status, UNCHANGED_V2);

            let mut converted = v1.clone();
            rename(&mut converted, Direction::FromV1);
            assert_eq!(converted, current);

            rename(&mut converted, Direction::ToV1);
            assert_eq!(converted, v1);
        }
    }

    #[test]
    fn renames_errors_nested_in_move_authentication_failures() {
        let mut error =
            json!({ "MoveAuthenticationError": { "error": "ExecutionCancelledDueToRandomnessUnavailable" } });
        rename_execution_error(&mut error, Direction::FromV1);
        assert_eq!(
            error,
            json!({ "MoveAuthentication": { "error": "ExecutionCanceledDueToRandomnessUnavailable" } })
        );
    }

    #[test]
    fn leaves_identifiers_outside_the_affected_places_unchanged() {
        let mut proof = json!({
            "targets": { "objects": [{ "Cancelled": "ExecutionCancelledDueToRandomnessUnavailable" }] },
            "transaction_proof": {
                "transaction": { "data": { "kind": { "Cancelled": { "cancelled_transactions": [] } } } },
                "effects": { "V1": { "status": { "success": true }, "changed_objects": [{ "kind": { "Cancelled": {} } }] } },
                "events": [{ "type": "0x2::example::CancelledTransactions" }]
            }
        });
        let original = proof.clone();

        rename(&mut proof, Direction::FromV1);
        assert_eq!(proof, original);
        rename(&mut proof, Direction::ToV1);
        assert_eq!(proof, original);
    }

    #[test]
    fn keeps_the_key_order() {
        let mut value = json!({ "a": 1, "Cancelled": 2, "z": 3 });
        rename_key(&mut value, ("Cancelled", "Canceled"));
        assert_eq!(serde_json::to_string(&value).unwrap(), r#"{"a":1,"Canceled":2,"z":3}"#);
    }
}
