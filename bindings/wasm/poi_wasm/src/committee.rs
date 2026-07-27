// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use fastcrypto::traits::ToFromBytes;
use iota_types::{base_types::AuthorityName, committee::Committee};
use serde::Deserialize;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::{
    proof::error_to_js,
    source::{BridgeError, LedgerSource},
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, catch, structural)]
    async fn committee(this: &LedgerSource, epoch: u64) -> Result<JsValue, JsValue>;
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
    /// Returns the epoch governed by this committee.
    #[wasm_bindgen(getter)]
    pub fn epoch(&self) -> u64 {
        self.0.epoch()
    }
}

/// Resolves the committee required to verify a Proof of Inclusion proof.
///
/// Node mode trusts the JavaScript source for committee data. Anchored mode
/// reserves the API for future genesis-authenticated committee walking.
#[wasm_bindgen(js_name = CommitteeResolver)]
pub struct WasmCommitteeResolver {
    source: LedgerSource,
    mode: CommitteeResolution,
}

enum CommitteeResolution {
    Node,
    Anchor(Committee),
}

#[wasm_bindgen(js_class = CommitteeResolver)]
impl WasmCommitteeResolver {
    /// Creates a trusted-node resolver backed by a JavaScript ledger source.
    #[wasm_bindgen(constructor)]
    pub fn new(source: LedgerSource) -> Self {
        Self::node(source)
    }

    /// Creates a resolver that trusts the JavaScript source for committee data.
    pub fn node(source: LedgerSource) -> Self {
        Self {
            source,
            mode: CommitteeResolution::Node,
        }
    }

    /// Creates a resolver anchored at an already trusted committee.
    ///
    /// Genesis-anchored committee walking is not implemented yet. The method
    /// reserves the public API that will delegate to `poi-rs` once the updated
    /// epoch-close resolver is integrated.
    pub fn anchor(source: LedgerSource, committee: &WasmCommittee) -> Self {
        Self {
            source,
            mode: CommitteeResolution::Anchor(committee.0.clone()),
        }
    }

    /// Resolves the committee governing `epoch`.
    pub async fn resolve(&self, epoch: u64) -> Result<WasmCommittee, JsValue> {
        if let CommitteeResolution::Anchor(committee) = &self.mode {
            return Err(js_sys::Error::new(&format!(
                "genesis-anchored committee resolution from epoch {} is not implemented yet",
                committee.epoch()
            ))
            .into());
        }

        let value = self
            .source
            .committee(epoch)
            .await
            .map_err(BridgeError::from_js)
            .map_err(error_to_js)?;
        let evidence: JsCommittee =
            serde_wasm_bindgen::from_value(value).map_err(|source| error_to_js(BridgeError(source.to_string())))?;

        decode_committee(epoch, evidence)
            .map(WasmCommittee)
            .map_err(error_to_js)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsCommittee {
    members: Vec<JsCommitteeMember>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsCommitteeMember {
    public_key: Vec<u8>,
    weight: u64,
}

fn decode_committee(requested_epoch: u64, evidence: JsCommittee) -> Result<Committee, BridgeError> {
    let voting_rights = evidence
        .members
        .into_iter()
        .map(|member| {
            AuthorityName::from_bytes(&member.public_key)
                .map(|authority| (authority, member.weight))
                .map_err(|source| BridgeError(format!("invalid committee public key: {source}")))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    Ok(Committee::new(requested_epoch, voting_rights))
}
