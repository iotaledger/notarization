// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trails::AuditTrailClientReadOnly;
use iota_interaction_ts::bindings::WasmIotaClient;
use iota_interaction_ts::wasm_error::{Result, WasmResult};
use product_common::bindings::utils::parse_wasm_object_id;
use product_common::bindings::WasmObjectID;
use product_common::core_client::CoreClientReadOnly;
use wasm_bindgen::prelude::*;

use crate::trail_handle::WasmAuditTrailHandle;

#[derive(Clone)]
#[wasm_bindgen(js_name = AuditTrailClientReadOnly)]
pub struct WasmAuditTrailClientReadOnly(pub(crate) AuditTrailClientReadOnly);

#[wasm_bindgen(js_class = AuditTrailClientReadOnly)]
impl WasmAuditTrailClientReadOnly {
    #[wasm_bindgen(js_name = create)]
    pub async fn new(iota_client: WasmIotaClient) -> Result<WasmAuditTrailClientReadOnly> {
        let client = AuditTrailClientReadOnly::new(iota_client).await.wasm_result()?;
        Ok(Self(client))
    }

    #[wasm_bindgen(js_name = createWithPkgId)]
    pub async fn new_with_pkg_id(
        iota_client: WasmIotaClient,
        package_id: WasmObjectID,
    ) -> Result<WasmAuditTrailClientReadOnly> {
        let package_id = parse_wasm_object_id(&package_id)?;
        let client = AuditTrailClientReadOnly::new_with_pkg_id(iota_client, package_id)
            .await
            .wasm_result()?;
        Ok(Self(client))
    }

    #[wasm_bindgen(js_name = packageId)]
    pub fn package_id(&self) -> String {
        self.0.package_id().to_string()
    }

    #[wasm_bindgen(js_name = packageHistory)]
    pub fn package_history(&self) -> Vec<String> {
        self.0
            .package_history()
            .into_iter()
            .map(|pkg_id| pkg_id.to_string())
            .collect()
    }

    #[wasm_bindgen]
    pub fn network(&self) -> String {
        self.0.network().to_string()
    }

    #[wasm_bindgen(js_name = chainId)]
    pub fn chain_id(&self) -> String {
        self.0.chain_id().to_string()
    }

    #[wasm_bindgen(js_name = iotaClient)]
    pub fn iota_client(&self) -> WasmIotaClient {
        self.0.iota_client().clone().into_inner()
    }

    pub fn trail(&self, trail_id: WasmObjectID) -> Result<WasmAuditTrailHandle> {
        let trail_id = parse_wasm_object_id(&trail_id)?;
        Ok(WasmAuditTrailHandle::from_read_only(self.0.clone(), trail_id))
    }
}
