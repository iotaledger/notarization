// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Bidirectional correction tracking for audit records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordCorrection {
    pub replaces: HashSet<u64>,
    pub is_replaced_by: Option<u64>,
}

impl RecordCorrection {
    pub fn new() -> Self {
        Self {
            replaces: HashSet::new(),
            is_replaced_by: None,
        }
    }

    pub fn with_replaces(replaces: HashSet<u64>) -> Self {
        Self {
            replaces,
            is_replaced_by: None,
        }
    }

    pub fn is_correction(&self) -> bool {
        !self.replaces.is_empty()
    }

    pub fn is_replaced(&self) -> bool {
        self.is_replaced_by.is_some()
    }
}
