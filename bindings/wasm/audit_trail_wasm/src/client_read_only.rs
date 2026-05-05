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

/// Package-ID overrides used when targeting custom audit-trail deployments.
///
/// @remarks
/// Pass an instance of this type to
/// {@link AuditTrailClientReadOnly.createWithPackageOverrides} or
/// {@link AuditTrailClient.createFromIotaClientWithPackageOverrides} when the connected network
/// hosts the audit-trail package — and optionally the `tf_components` package — at addresses that
/// are not part of the SDK's built-in registry. Leave a field unset to fall back to the registry
/// lookup for that package.
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
    ///
    /// @param auditTrailPackageId - Optional audit-trail package ID to use instead of the registry
    /// entry.
    /// @param tfComponentsPackageId - Optional `tf_components` package ID to use instead of the
    /// registry entry.
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

/// Read-only audit-trail client.
///
/// @remarks
/// This is the main entry point for package resolution and typed reads. Use
/// {@link AuditTrailClientReadOnly.trail} to obtain an {@link AuditTrailHandle} bound to a single
/// trail object.
#[derive(Clone)]
#[wasm_bindgen(js_name = AuditTrailClientReadOnly)]
pub struct WasmAuditTrailClientReadOnly(pub(crate) AuditTrailClientReadOnly);

#[wasm_bindgen(js_class = AuditTrailClientReadOnly)]
impl WasmAuditTrailClientReadOnly {
    /// Creates a read-only client by resolving package IDs from the connected network.
    ///
    /// @remarks
    /// This is the recommended constructor for official deployments tracked by the built-in
    /// package registry.
    ///
    /// @param iotaClient - IOTA client used to talk to the network.
    ///
    /// @returns A read-only audit-trail client bound to the resolved package IDs.
    ///
    /// @throws When package resolution fails for the connected network.
    #[wasm_bindgen(js_name = create)]
    pub async fn new(iota_client: WasmIotaClient) -> Result<WasmAuditTrailClientReadOnly> {
        let client = AuditTrailClientReadOnly::new(iota_client).await.wasm_result()?;
        Ok(Self(client))
    }

    /// Creates a read-only client with explicit package overrides.
    ///
    /// @remarks
    /// Prefer this when targeting a local deployment, preview environment, or any package pair
    /// that is not yet part of the SDK's built-in registry.
    ///
    /// @param iotaClient - IOTA client used to talk to the network.
    /// @param packageOverrides - Package IDs to use instead of registry lookups.
    ///
    /// @returns A read-only audit-trail client bound to the supplied package IDs.
    ///
    /// @throws When the supplied package IDs are malformed or cannot be resolved.
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
    /// @remarks
    /// Compatibility helper for callers that need exactly one package override.
    ///
    /// @param iotaClient - IOTA client used to talk to the network.
    /// @param packageId - Audit-trail package ID to use instead of the registry entry.
    ///
    /// @returns A read-only audit-trail client bound to `packageId`.
    ///
    /// @throws When `packageId` is malformed or cannot be resolved.
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

    /// Returns the audit-trail package ID currently in use.
    ///
    /// @returns Stringified object ID of the resolved audit-trail package.
    #[wasm_bindgen(js_name = packageId)]
    pub fn package_id(&self) -> String {
        self.0.package_id().to_string()
    }

    /// Returns the `tf_components` package ID currently in use.
    ///
    /// @returns Stringified object ID of the resolved `tf_components` package.
    #[wasm_bindgen(js_name = tfComponentsPackageId)]
    pub fn tf_components_package_id(&self) -> String {
        self.0.tf_components_package_id().to_string()
    }

    /// Returns the resolved audit-trail package upgrade history.
    ///
    /// @returns Stringified object IDs of every published version, most recent first.
    #[wasm_bindgen(js_name = packageHistory)]
    pub fn package_history(&self) -> Vec<String> {
        self.0
            .package_history()
            .into_iter()
            .map(|pkg_id| pkg_id.to_string())
            .collect()
    }

    /// Returns the human-readable name of the network this client is connected to.
    ///
    /// @returns Network name (e.g. `"mainnet"`, `"testnet"`, `"localnet"`).
    #[wasm_bindgen]
    pub fn network(&self) -> String {
        self.0.network().to_string()
    }

    /// Returns the chain ID of the network this client is connected to.
    ///
    /// @returns Hex-encoded chain identifier.
    #[wasm_bindgen(js_name = chainId)]
    pub fn chain_id(&self) -> String {
        self.0.chain_id().to_string()
    }

    /// Returns the underlying IOTA client used to talk to the network.
    ///
    /// @returns The IOTA client passed to (or constructed during) creation of this client.
    #[wasm_bindgen(js_name = iotaClient)]
    pub fn iota_client(&self) -> WasmIotaClient {
        self.0.iota_client().clone().into_inner()
    }

    /// Returns a trail-scoped handle for the given trail object ID.
    ///
    /// @remarks
    /// Creating the handle is cheap. Reads only happen when methods are called on the returned
    /// handle.
    ///
    /// @param trailId - Object ID of the trail this handle should target.
    ///
    /// @returns Read-only {@link AuditTrailHandle} bound to `trailId`.
    ///
    /// @throws When `trailId` is not a valid object ID.
    pub fn trail(&self, trail_id: WasmObjectID) -> Result<WasmAuditTrailHandle> {
        let trail_id = parse_wasm_object_id(&trail_id)?;
        Ok(WasmAuditTrailHandle::from_read_only(self.0.clone(), trail_id))
    }
}
