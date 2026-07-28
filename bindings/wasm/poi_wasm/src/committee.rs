// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_types::committee::Committee;
use poi_rs::CommitteeResolver;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::{
    error::WasmResult,
    source::{LedgerSource, SourceAdapter},
};

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
/// authenticates committee lineage from a trusted committee and caches verified
/// committees in memory.
#[wasm_bindgen(js_name = CommitteeResolver)]
pub struct WasmCommitteeResolver(CommitteeResolver<SourceAdapter>);

#[wasm_bindgen(js_class = CommitteeResolver)]
impl WasmCommitteeResolver {
    /// Creates a trusted-node resolver backed by a JavaScript ledger source.
    #[wasm_bindgen(constructor)]
    pub fn new(source: LedgerSource) -> Self {
        Self::node(source)
    }

    /// Creates a resolver that trusts the JavaScript source for committee data.
    pub fn node(source: LedgerSource) -> Self {
        Self(CommitteeResolver::node(SourceAdapter::new(source)))
    }

    /// Creates a resolver anchored at an already trusted committee.
    ///
    /// The resolver authenticates every epoch-close checkpoint from the trusted
    /// committee up to the requested epoch.
    pub fn anchor(source: LedgerSource, committee: &WasmCommittee) -> Self {
        Self(CommitteeResolver::anchor(
            SourceAdapter::new(source),
            committee.0.clone(),
        ))
    }

    /// Resolves the committee governing `epoch`.
    pub async fn resolve(&self, epoch: u64) -> Result<WasmCommittee, JsValue> {
        self.0.resolve(epoch).await.map(WasmCommittee).wasm_result()
    }
}
