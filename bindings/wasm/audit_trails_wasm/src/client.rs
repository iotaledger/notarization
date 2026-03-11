// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction_ts::bindings::{WasmIotaClient, WasmPublicKey, WasmTransactionSigner};
use iota_interaction_ts::wasm_error::Result;
use product_common::bindings::WasmObjectID;
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use wasm_bindgen::prelude::*;

use crate::builder::WasmAuditTrailBuilder;
use crate::client_read_only::WasmAuditTrailClientReadOnly;
use crate::trail_handle::WasmAuditTrailHandle;
use crate::audit_trails_wasm_result;

#[derive(Clone)]
#[wasm_bindgen(js_name = AuditTrailClient)]
pub struct WasmAuditTrailClient(pub(crate) audit_trails::AuditTrailClient<WasmTransactionSigner>);

#[wasm_bindgen(js_class = AuditTrailClient)]
impl WasmAuditTrailClient {
    #[wasm_bindgen(js_name = create)]
    pub async fn new(
        client: WasmAuditTrailClientReadOnly,
        signer: WasmTransactionSigner,
    ) -> Result<WasmAuditTrailClient> {
        let client = audit_trails_wasm_result(audit_trails::AuditTrailClient::new(client.0, signer).await)?;
        Ok(Self(client))
    }

    #[wasm_bindgen(js_name = senderPublicKey)]
    pub fn sender_public_key(&self) -> Result<WasmPublicKey> {
        self.0.public_key().try_into()
    }

    #[wasm_bindgen(js_name = senderAddress)]
    pub fn sender_address(&self) -> String {
        self.0.address().to_string()
    }

    #[wasm_bindgen]
    pub fn network(&self) -> String {
        self.0.network().to_string()
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

    #[wasm_bindgen(js_name = iotaClient)]
    pub fn iota_client(&self) -> WasmIotaClient {
        self.0.read_only().iota_client().clone().into_inner()
    }

    #[wasm_bindgen]
    pub fn signer(&self) -> WasmTransactionSigner {
        self.0.signer().clone()
    }

    #[wasm_bindgen(js_name = readOnly)]
    pub fn read_only(&self) -> WasmAuditTrailClientReadOnly {
        WasmAuditTrailClientReadOnly(self.0.read_only().clone())
    }

    #[wasm_bindgen(js_name = createTrail)]
    pub fn create_trail(&self) -> WasmAuditTrailBuilder {
        WasmAuditTrailBuilder(self.0.create_trail())
    }

    pub fn trail(&self, trail_id: WasmObjectID) -> Result<WasmAuditTrailHandle> {
        let trail_id = product_common::bindings::utils::parse_wasm_object_id(&trail_id)?;
        Ok(WasmAuditTrailHandle::from_full(self.0.clone(), trail_id))
    }
}
