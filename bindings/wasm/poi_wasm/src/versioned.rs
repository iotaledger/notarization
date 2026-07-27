// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_sdk_types::Object;
use iota_sdk_types::{CheckpointSummary, Event, ValidatorAggregatedSignature};
use serde::{Deserialize, Serialize};

// These one-variant envelopes match the BCS version discriminants used by the
// IOTA gRPC API. The inner values remain the canonical iota-sdk-types values.
#[derive(Deserialize, Serialize)]
pub(crate) enum VersionedObject {
    V1(Object),
}

#[derive(Deserialize, Serialize)]
pub(crate) enum VersionedEvent {
    V1(Event),
}

#[derive(Deserialize, Serialize)]
pub(crate) enum VersionedCheckpointSummary {
    V1(CheckpointSummary),
}

#[derive(Deserialize, Serialize)]
pub(crate) enum VersionedValidatorAggregatedSignature {
    V1(ValidatorAggregatedSignature),
}
