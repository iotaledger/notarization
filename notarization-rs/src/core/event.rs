// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::ObjectID;
use serde::{Deserialize, Serialize};
/// An event that can be emitted by the ITH.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event<D> {
    #[serde(flatten)]
    pub data: D,
}

/// An event that is emitted when a new dynamic notarization is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicNotarizationCreated {
    pub notarization_id: ObjectID,
}

/// An event that is emitted when a new dynamic notarization is updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedNotarizationCreated {
    pub notarization_id: ObjectID,
}
