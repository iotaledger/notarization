// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

use super::timelock::LockMetadata;

/// Immutable provenance fields of a notarization, fixed at creation time.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImmutableMetadata {
    /// Creation timestamp, in milliseconds since the Unix epoch.
    pub created_at: u64,
    /// Human-readable description of the notarization.
    pub description: Option<String>,
    /// Optional lock metadata.
    ///
    /// Presence depends on the Notarization Method:
    /// * `Dynamic`: absent when the Dynamic-Notarization carries no transfer lock; present otherwise.
    /// * `Locked`: always present.
    pub locking: Option<LockMetadata>,
}
