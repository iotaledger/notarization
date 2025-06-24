// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::id::UID;
use serde::{Deserialize, Serialize};

use super::NotarizationMethod;
use super::metadata::ImmutableMetadata;
use super::state::State;

/// A notarization record stored on the blockchain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OnChainNotarization {
    /// The unique identifier of the notarization.
    pub id: UID,
    /// The state of the notarization.
    pub state: State,
    /// The immutable metadata of the notarization.
    pub immutable_metadata: ImmutableMetadata,
    /// The updatable metadata of the notarization.
    pub updatable_metadata: Option<String>,
    /// The timestamp of the last state change.
    pub last_state_change_at: u64,
    /// The number of state changes.
    pub state_version_count: u64,
    /// The method of the notarization.
    pub method: NotarizationMethod,
}
