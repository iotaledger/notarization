// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::timelock::LockMetadata;

/// The immutable metadata of a notarization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImmutableMetadata {
    /// Timestamp when the `Notarization` was created
    pub created_at: u64,
    /// Description of the `Notarization`
    pub description: Option<String>,
    /// Optional lock metadata for `Notarization`
    pub locking: Option<LockMetadata>,
}
