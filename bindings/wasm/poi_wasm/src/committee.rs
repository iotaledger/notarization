// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use fastcrypto::traits::ToFromBytes;
use iota_types::{base_types::AuthorityName, committee::Committee};
use js_sys::Promise;
use serde::Deserialize;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::{
    proof::error_to_js,
    source::{BridgeError, LedgerSource, SourceAdapter},
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(method, catch, structural)]
    fn committee(this: &LedgerSource, epoch: u64) -> Result<Promise, JsValue>;
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

/// Resolves committees reported by a node inside the caller's trust boundary.
///
/// This resolver does not authenticate committee lineage from genesis. The
/// connected node is authoritative for the committee returned for each epoch.
#[wasm_bindgen(js_name = CommitteeResolver)]
pub struct WasmCommitteeResolver {
    source: LedgerSource,
}

#[wasm_bindgen(js_class = CommitteeResolver)]
impl WasmCommitteeResolver {
    /// Creates a trusted-node resolver backed by a JavaScript ledger source.
    #[wasm_bindgen(constructor)]
    pub fn new(source: LedgerSource) -> Self {
        Self { source }
    }

    /// Returns the committee reported by the trusted node for `epoch`.
    pub async fn resolve(&self, epoch: u64) -> Result<WasmCommittee, JsValue> {
        let value = SourceAdapter::await_method(self.source.committee(epoch))
            .await
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
