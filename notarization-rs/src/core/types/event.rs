// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::ObjectID;
use serde::{Deserialize, Serialize};
/// An event emitted by notarization operations.
///
/// Generic wrapper for different event data types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event<D> {
    #[serde(flatten)]
    pub data: D,
}

/// An event that is emitted when a new dynamic notarization is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DynamicNotarizationCreated {
    pub notarization_id: ObjectID,
}

/// An event that is emitted when a new locked notarization is created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LockedNotarizationCreated {
    pub notarization_id: ObjectID,
}
