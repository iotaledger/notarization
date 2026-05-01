// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trail::{AuditTrailClientReadOnly, PackageOverrides};
use iota_interaction_ts::bindings::WasmIotaClient;
use iota_interaction_ts::wasm_error::{Result, WasmResult};
use product_common::bindings::utils::parse_wasm_object_id;
use product_common::bindings::WasmObjectID;
use product_common::core_client::CoreClientReadOnly;
use wasm_bindgen::prelude::*;

use crate::trail_handle::WasmAuditTrailHandle;

/// Package-ID overrides exposed to JavaScript and TypeScript consumers.
#[derive(Clone)]
#[wasm_bindgen(js_name = PackageOverrides, getter_with_clone, inspectable)]
pub struct WasmPackageOverrides {
    /// Override for the audit-trail package ID.
    #[wasm_bindgen(js_name = auditTrailPackageId)]
    pub audit_trail_package_id: Option<WasmObjectID>,
    /// Override for the `tf_components` package ID.
    #[wasm_bindgen(js_name = tfComponentsPackageId)]
    pub tf_components_package_id: Option<WasmObjectID>,
}

#[wasm_bindgen(js_class = PackageOverrides)]
impl WasmPackageOverrides {
    /// Creates package overrides for custom deployments.
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
            audit_trail: value
                .audit_trail_package_id
                .as_ref()
                .map(parse_wasm_object_id)
                .transpose()?,
            tf_component: value
                .tf_components_package_id
                .as_ref()
                .map(parse_wasm_object_id)
                .transpose()?,
        })
    }
}

/// Read-only audit-trail client exposed to wasm consumers.
///
/// This is the main JS/TS entry point for package resolution and typed reads. Use [`Self::trail`]
/// to get an [`AuditTrailHandle`](crate::trail_handle::WasmAuditTrailHandle) bound to one trail
/// object.
#[derive(Clone)]
#[wasm_bindgen(js_name = AuditTrailClientReadOnly)]
pub struct WasmAuditTrailClientReadOnly(pub(crate) AuditTrailClientReadOnly);

#[wasm_bindgen(js_class = AuditTrailClientReadOnly)]
impl WasmAuditTrailClientReadOnly {
    /// Creates a read-only client by resolving package IDs from the connected network.
    ///
    /// This is the recommended constructor for official deployments tracked by the built-in
    /// package registry.
    #[wasm_bindgen(js_name = create)]
    pub async fn new(iota_client: WasmIotaClient) -> Result<WasmAuditTrailClientReadOnly> {
        let client = AuditTrailClientReadOnly::new(iota_client).await.wasm_result()?;
        Ok(Self(client))
    }

    /// Creates a read-only client with explicit package overrides.
    ///
    /// Prefer this when your JS/TS app talks to a local deployment, preview environment, or any
    /// package pair that is not yet part of the registry baked into the SDK.
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

    /// Creates a read-only client while overriding only the audit-trail package ID.
    ///
    /// This is a compatibility helper for existing callers that only need a single package
    /// override.
    #[wasm_bindgen(js_name = createWithPkgId)]
    pub async fn new_with_pkg_id(
        iota_client: WasmIotaClient,
        package_id: WasmObjectID,
    ) -> Result<WasmAuditTrailClientReadOnly> {
        let package_id = parse_wasm_object_id(&package_id)?;
        let client = AuditTrailClientReadOnly::new_with_package_overrides(
            iota_client,
            PackageOverrides {
                audit_trail: Some(package_id),
                tf_component: None,
            },
        )
        .await
        .wasm_result()?;
        Ok(Self(client))
    }

    /// Returns the audit-trail package ID used by this client.
    #[wasm_bindgen(js_name = packageId)]
    pub fn package_id(&self) -> String {
        self.0.package_id().to_string()
    }

    /// Returns the `tf_components` package ID used by this client.
    #[wasm_bindgen(js_name = tfComponentsPackageId)]
    pub fn tf_components_package_id(&self) -> String {
        self.0.tf_components_package_id().to_string()
    }

    /// Returns the resolved audit-trail package history as stringified object IDs.
    #[wasm_bindgen(js_name = packageHistory)]
    pub fn package_history(&self) -> Vec<String> {
        self.0
            .package_history()
            .into_iter()
            .map(|pkg_id| pkg_id.to_string())
            .collect()
    }

    /// Returns the connected network name.
    #[wasm_bindgen]
    pub fn network(&self) -> String {
        self.0.network().to_string()
    }

    /// Returns the connected chain ID.
    #[wasm_bindgen(js_name = chainId)]
    pub fn chain_id(&self) -> String {
        self.0.chain_id().to_string()
    }

    /// Returns the underlying IOTA client wrapper.
    #[wasm_bindgen(js_name = iotaClient)]
    pub fn iota_client(&self) -> WasmIotaClient {
        self.0.iota_client().clone().into_inner()
    }

    /// Returns a trail-scoped handle for the given trail object ID.
    ///
    /// Creating the handle is cheap. Reads only happen when you call methods on the returned
    /// handle.
    pub fn trail(&self, trail_id: WasmObjectID) -> Result<WasmAuditTrailHandle> {
        let trail_id = parse_wasm_object_id(&trail_id)?;
        Ok(WasmAuditTrailHandle::from_read_only(self.0.clone(), trail_id))
    }
}
