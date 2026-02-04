// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// Defines a locking window (time or count based).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockingWindow {
    pub time_window_seconds: Option<u64>,
    pub count_window: Option<u64>,
}

impl LockingWindow {
    pub fn none() -> Self {
        Self {
            time_window_seconds: None,
            count_window: None,
        }
    }

    pub fn time_based(seconds: u64) -> Self {
        Self {
            time_window_seconds: Some(seconds),
            count_window: None,
        }
    }

    pub fn count_based(count: u64) -> Self {
        Self {
            time_window_seconds: None,
            count_window: Some(count),
        }
    }
}

/// Locking configuration for the audit trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockingConfig {
    pub delete_record_lock: LockingWindow,
}

impl LockingConfig {
    pub fn none() -> Self {
        Self {
            delete_record_lock: LockingWindow::none(),
        }
    }

    pub fn time_based(seconds: u64) -> Self {
        Self {
            delete_record_lock: LockingWindow::time_based(seconds),
        }
    }

    pub fn count_based(count: u64) -> Self {
        Self {
            delete_record_lock: LockingWindow::count_based(count),
        }
    }
}
