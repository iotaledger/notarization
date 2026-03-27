// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trails::{AuditTrailClientReadOnly, PackageOverrides};
use iota_interaction_ts::bindings::WasmIotaClient;
use iota_interaction_ts::wasm_error::{Result, WasmResult};
use product_common::bindings::utils::parse_wasm_object_id;
use product_common::bindings::WasmObjectID;
use product_common::core_client::CoreClientReadOnly;
use wasm_bindgen::prelude::*;

use crate::trail_handle::WasmAuditTrailHandle;

#[derive(Clone)]
#[wasm_bindgen(js_name = PackageOverrides, getter_with_clone, inspectable)]
pub struct WasmPackageOverrides {
    #[wasm_bindgen(js_name = auditTrailPackageId)]
    pub audit_trail_package_id: Option<WasmObjectID>,
    #[wasm_bindgen(js_name = tfComponentsPackageId)]
    pub tf_components_package_id: Option<WasmObjectID>,
}

#[wasm_bindgen(js_class = PackageOverrides)]
impl WasmPackageOverrides {
    #[wasm_bindgen(constructor)]
    pub fn new(
        audit_trail_package_id: Option<WasmObjectID>,
        tf_components_package_id: Option<WasmObjectID>,
    ) -> WasmPackageOverrides {
        Self {
            audit_trail_package_id,
            tf_components_package_id,
        }
    }
}

impl TryFrom<WasmPackageOverrides> for PackageOverrides {
    type Error = JsValue;

    fn try_from(value: WasmPackageOverrides) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            audit_trail_package_id: value
                .audit_trail_package_id
                .as_ref()
                .map(parse_wasm_object_id)
                .transpose()?,
            tf_components_package_id: value
                .tf_components_package_id
                .as_ref()
                .map(parse_wasm_object_id)
                .transpose()?,
        })
    }
}

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

    #[wasm_bindgen(js_name = createWithPackageOverrides)]
    pub async fn new_with_package_overrides(
        iota_client: WasmIotaClient,
        package_overrides: WasmPackageOverrides,
    ) -> Result<WasmAuditTrailClientReadOnly> {
        let package_overrides = PackageOverrides::try_from(package_overrides)?;
        let client = AuditTrailClientReadOnly::new_with_package_overrides(iota_client, package_overrides)
            .await
            .wasm_result()?;
        Ok(Self(client))
    }

    #[wasm_bindgen(js_name = createWithPkgId)]
    pub async fn new_with_pkg_id(
        iota_client: WasmIotaClient,
        package_id: WasmObjectID,
    ) -> Result<WasmAuditTrailClientReadOnly> {
        let package_id = parse_wasm_object_id(&package_id)?;
        let client = AuditTrailClientReadOnly::new_with_package_overrides(
            iota_client,
            PackageOverrides {
                audit_trail_package_id: Some(package_id),
                tf_components_package_id: None,
            },
        )
        .await
        .wasm_result()?;
        Ok(Self(client))
    }

    #[wasm_bindgen(js_name = packageId)]
    pub fn package_id(&self) -> String {
        self.0.package_id().to_string()
    }

    #[wasm_bindgen(js_name = tfComponentsPackageId)]
    pub fn tf_components_package_id(&self) -> String {
        self.0.tf_components_package_id().to_string()
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
