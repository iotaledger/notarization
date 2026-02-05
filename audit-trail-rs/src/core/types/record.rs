// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::IotaAddress;
use serde::{Deserialize, Serialize};

use super::record_correction::RecordCorrection;

/// Supported record data types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordData {
    Bytes(Vec<u8>),
    Text(String),
}

impl RecordData {
    pub fn bytes(data: impl Into<Vec<u8>>) -> Self {
        Self::Bytes(data.into())
    }

    pub fn text(data: impl Into<String>) -> Self {
        Self::Text(data.into())
    }
}

/// A single record in the audit trail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record<D = RecordData> {
    pub data: D,
    pub metadata: Option<String>,
    pub sequence_number: u64,
    pub added_by: IotaAddress,
    pub added_at: u64,
    pub correction: RecordCorrection,
}
