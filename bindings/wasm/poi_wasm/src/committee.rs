// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use iota_types::base_types::AuthorityName;
use iota_types::committee::{Committee, EpochId, StakeUnit, TOTAL_VOTING_POWER};
use js_sys::Uint8Array;
use poi_rs::{CommitteeResolution, CommitteeResolver};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::error::{PoiError, WasmResult};
use crate::proof::{WasmProof, WasmVerifiedProof};
use crate::source::LedgerSource;

#[derive(Deserialize, Serialize)]
struct CommitteeJson {
    epoch: EpochId,
    voting_rights: Vec<(AuthorityName, StakeUnit)>,
}

/// A validator committee used to verify a Proof of Inclusion proof.
#[wasm_bindgen(js_name = Committee)]
pub struct WasmCommittee(Committee);

impl WasmCommittee {
    pub(crate) const fn inner(&self) -> &Committee {
        &self.0
    }
}

#[wasm_bindgen(js_class = Committee)]
impl WasmCommittee {
    /// Deserializes and validates a committee from its Rust JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(json: &str) -> WasmResult<WasmCommittee> {
        let committee: CommitteeJson = serde_json::from_str(json)
            .map_err(|error| PoiError::invalid_input(format!("invalid committee JSON: {error}")))?;
        let mut voting_rights = BTreeMap::new();
        let mut total_voting_power = 0_u64;

        for (authority, voting_power) in committee.voting_rights {
            if voting_rights.insert(authority, voting_power).is_some() {
                return Err(PoiError::invalid_input("committee contains a duplicate authority").into());
            }
            total_voting_power = total_voting_power
                .checked_add(voting_power)
                .ok_or_else(|| PoiError::invalid_input("committee voting power exceeds the supported range"))?;
        }

        if total_voting_power != TOTAL_VOTING_POWER {
            return Err(PoiError::invalid_input(format!(
                "committee voting power must total {TOTAL_VOTING_POWER}, received {total_voting_power}"
            ))
            .into());
        }

        Ok(WasmCommittee(Committee::new(committee.epoch, voting_rights)))
    }

    /// Serializes this committee for persistence and later restoration with `fromJSON`.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> WasmResult<String> {
        let committee = CommitteeJson {
            epoch: self.0.epoch,
            voting_rights: self.0.voting_rights.clone(),
        };

        Ok(serde_json::to_string(&committee)?)
    }

    /// Returns the epoch governed by this committee.
    #[wasm_bindgen(getter)]
    pub fn epoch(&self) -> u64 {
        self.0.epoch()
    }
}

/// Selects how a resolver establishes trust in committee data.
#[wasm_bindgen(js_name = CommitteeResolution)]
pub struct WasmCommitteeResolution(CommitteeResolution);

#[wasm_bindgen(js_class = CommitteeResolution)]
impl WasmCommitteeResolution {
    /// Accepts committee data returned directly by the JavaScript source.
    ///
    /// This does not authenticate committee lineage. Use it only when the
    /// source is inside the caller's trust boundary.
    #[wasm_bindgen(js_name = trustedNode)]
    pub fn trusted_node() -> Self {
        Self(CommitteeResolution::TrustedNode)
    }

    /// Authenticates committee lineage from an already trusted committee.
    pub fn anchored(committee: &WasmCommittee) -> Self {
        Self(CommitteeResolution::anchored(committee.0.clone()))
    }

    /// Authenticates committee lineage from the committee in a trusted genesis blob.
    #[wasm_bindgen(js_name = fromGenesis)]
    pub fn from_genesis(genesis_blob: Uint8Array) -> WasmResult<Self> {
        let bytes = genesis_blob.to_vec();
        let resolution = CommitteeResolution::from_genesis(bytes.as_slice())
            .map_err(|error| PoiError::invalid_input(format!("failed to load trusted genesis blob: {error}")))?;

        Ok(Self(resolution))
    }
}

/// Resolves the committee required to verify a Proof of Inclusion proof.
///
/// Node mode trusts the JavaScript source for committee data. Anchored mode
/// authenticates committee lineage from a trusted committee and caches verified
/// committees in memory.
#[wasm_bindgen(js_name = CommitteeResolver)]
pub struct WasmCommitteeResolver(CommitteeResolver<LedgerSource>);

#[wasm_bindgen(js_class = CommitteeResolver)]
impl WasmCommitteeResolver {
    /// Creates a resolver backed by a JavaScript ledger source.
    #[wasm_bindgen(constructor)]
    pub fn new(source: LedgerSource, resolution: &WasmCommitteeResolution) -> Self {
        Self(CommitteeResolver::new(source, resolution.0.clone()))
    }

    /// Resolves the committee governing `epoch`.
    pub async fn resolve(&self, epoch: u64) -> WasmResult<WasmCommittee> {
        let committee = self.0.resolve(epoch).await?;

        Ok(WasmCommittee(committee))
    }

    /// Resolves the committee required by `proof` and returns its authenticated targets.
    pub async fn verify(&self, proof: &WasmProof) -> WasmResult<WasmVerifiedProof> {
        let verified = self.0.verify(&proof.0).await?;

        Ok(WasmVerifiedProof::new(&proof.0, verified))
    }
}
