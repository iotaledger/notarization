// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::IotaAddress;
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
    ///
    /// The `state` of a notarization contains the notarized data and metadata associated with
    /// the current version of the `state`.
    ///
    /// `state` can be updated depending on the used `NotarizationMethod`:
    /// - Dynamic: Can be updated anytime after notarization creation
    /// - Locked: Immutable after notarization creation
    ///
    /// Use `NotarizationClient::update_state()` for `state` updates.
    pub state: State,
    /// The immutable metadata of the notarization.
    ///
    /// NOTE:
    /// - provides immutable information, assertions and guaranties for third parties
    /// - `immutable_metadata` are automatically created at creation time and cannot be updated thereafter
    pub immutable_metadata: ImmutableMetadata,
    /// The updatable metadata of the notarization.
    ///
    /// Provides context or additional information for third parties
    ///
    /// `updatable_metadata` can be updated depending on the used `NotarizationMethod`:
    /// - Dynamic: Can be updated anytime after notarization creation
    /// - Locked: Immutable after notarization creation
    ///
    /// NOTE:
    /// - `updatable_metadata` can be updated independently of `state`
    /// - Updating `updatable_metadata` does not increase the `state_version_count`
    /// - Updating `updatable_metadata` does not change the `last_state_change_at` timestamp
    /// - Use `NotarizationClient::update_metadata()` for `updatable_metadata` updates.
    pub updatable_metadata: Option<String>,
    /// The timestamp of the last state change (milliseconds since UNIX epoch)
    pub last_state_change_at: u64,
    /// The number of state changes.
    pub state_version_count: u64,
    /// The method of the notarization.
    pub method: NotarizationMethod,
    /// The owner of the notarization.
    #[serde(skip)]
    pub owner: IotaAddress,
}
