// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use audit_trails::{AuditTrailClient, AuditTrailClientReadOnly, PackageOverrides};
use iota_interaction_ts::bindings::{WasmIotaClient, WasmPublicKey, WasmTransactionSigner};
use iota_interaction_ts::wasm_error::{wasm_error, Result, WasmResult};
use product_common::bindings::utils::parse_wasm_object_id;
use product_common::bindings::WasmObjectID;
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use wasm_bindgen::prelude::*;

use crate::builder::WasmAuditTrailBuilder;
use crate::client_read_only::{WasmAuditTrailClientReadOnly, WasmPackageOverrides};
use crate::trail_handle::WasmAuditTrailHandle;

#[derive(Clone)]
#[wasm_bindgen(js_name = AuditTrailClient)]
pub struct WasmAuditTrailClient(pub(crate) AuditTrailClient<WasmTransactionSigner>);

#[wasm_bindgen(js_class = AuditTrailClient)]
impl WasmAuditTrailClient {
    #[wasm_bindgen(js_name = create)]
    pub async fn new(
        client: WasmAuditTrailClientReadOnly,
        signer: WasmTransactionSigner,
    ) -> Result<WasmAuditTrailClient> {
        let client = AuditTrailClient::new(client.0, signer).await.wasm_result()?;
        Ok(Self(client))
    }

    #[wasm_bindgen(js_name = createFromIotaClient)]
    pub async fn create_from_iota_client(
        iota_client: WasmIotaClient,
        signer: WasmTransactionSigner,
        package_id: Option<WasmObjectID>,
    ) -> Result<WasmAuditTrailClient> {
        let read_only = if let Some(package_id) = package_id {
            let package_id = parse_wasm_object_id(&package_id)?;
            AuditTrailClientReadOnly::new_with_package_overrides(
                iota_client,
                PackageOverrides {
                    audit_trail_package_id: Some(package_id),
                    tf_components_package_id: None,
                },
            )
            .await
            .wasm_result()?
        } else {
            AuditTrailClientReadOnly::new(iota_client).await.wasm_result()?
        };

        let client = AuditTrailClient::new(read_only, signer).await.wasm_result()?;
        Ok(Self(client))
    }

    #[wasm_bindgen(js_name = createFromIotaClientWithPackageOverrides)]
    pub async fn create_from_iota_client_with_package_overrides(
        iota_client: WasmIotaClient,
        signer: WasmTransactionSigner,
        package_overrides: Option<WasmPackageOverrides>,
    ) -> Result<WasmAuditTrailClient> {
        let read_only = if let Some(package_overrides) = package_overrides {
            let package_overrides = PackageOverrides::try_from(package_overrides)?;
            AuditTrailClientReadOnly::new_with_package_overrides(iota_client, package_overrides)
                .await
                .wasm_result()?
        } else {
            AuditTrailClientReadOnly::new(iota_client).await.wasm_result()?
        };

        let client = AuditTrailClient::new(read_only, signer).await.wasm_result()?;
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

    #[wasm_bindgen(js_name = chainId)]
    pub fn chain_id(&self) -> String {
        self.0.chain_id().to_string()
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

    #[wasm_bindgen(js_name = iotaClient)]
    pub fn iota_client(&self) -> WasmIotaClient {
        self.0.read_only().iota_client().clone().into_inner()
    }

    #[wasm_bindgen]
    pub fn signer(&self) -> WasmTransactionSigner {
        self.0.signer().clone()
    }

    #[wasm_bindgen(js_name = withSigner)]
    pub async fn with_signer(self, signer: WasmTransactionSigner) -> Result<WasmAuditTrailClient> {
        let client = self
            .0
            .with_signer(signer)
            .await
            .map_err(|err| wasm_error(anyhow!(err.to_string())))?;
        Ok(Self(client))
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
        let trail_id = parse_wasm_object_id(&trail_id)?;
        Ok(WasmAuditTrailHandle::from_full(self.0.clone(), trail_id))
    }
}
