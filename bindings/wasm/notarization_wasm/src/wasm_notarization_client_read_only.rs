// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use anyhow::anyhow;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction_ts::bindings::WasmIotaClient;
use iota_interaction_ts::wasm_error::{wasm_error, Result, WasmResult};
use notarization::NotarizationClientReadOnly;
use product_common::bindings::utils::parse_wasm_object_id;
use product_common::bindings::WasmObjectID;
use product_common::core_client::CoreClientReadOnly;
use wasm_bindgen::prelude::*;

use crate::wasm_notarization::WasmOnChainNotarization;
use crate::wasm_types::{WasmLockMetadata, WasmNotarizationMethod, WasmState};

/// A client to interact with Notarization objects on the IOTA ledger.
///
/// This client is used for read-only operations, meaning it does not require an account
/// or signing capabilities. For write operations, use {@link NotarizationClient}.
#[derive(Clone)]
#[wasm_bindgen(js_name = NotarizationClientReadOnly)]
pub struct WasmNotarizationClientReadOnly(pub(crate) NotarizationClientReadOnly);

// Builder-related functions
#[wasm_bindgen(js_class = NotarizationClientReadOnly)]
impl WasmNotarizationClientReadOnly {
    /// Creates a new instance of `otarizationClientReadOnly`.
    ///
    /// # Arguments
    /// * `iota_client` - The IOTA client used for interacting with the ledger.
    ///
    /// # Returns
    /// A new `NotarizationClientReadOnly` instance.
    #[wasm_bindgen(js_name = create)]
    pub async fn new(iota_client: WasmIotaClient) -> Result<WasmNotarizationClientReadOnly> {
        let inner_client = NotarizationClientReadOnly::new(iota_client).await.map_err(wasm_error)?;
        Ok(WasmNotarizationClientReadOnly(inner_client))
    }

    /// Creates a new instance of `NotarizationClientReadOnly` using a specific package ID.
    ///
    /// # Arguments
    /// * `iota_client` - The IOTA client used for interacting with the ledger.
    /// * `iota_notarization_pkg_id` - The notarization package ID.
    ///
    /// # Returns
    /// A new `NotarizationClientReadOnly` instance.
    #[wasm_bindgen(js_name = createWithPkgId)]
    pub async fn new_new_with_pkg_id(
        iota_client: WasmIotaClient,
        iota_notarization_pkg_id: String,
    ) -> Result<WasmNotarizationClientReadOnly> {
        let inner_client = NotarizationClientReadOnly::new_with_pkg_id(
            iota_client,
            ObjectID::from_str(&iota_notarization_pkg_id)
                .map_err(|e| anyhow!("Could not parse iota_notarization_pkg_id: {}", e.to_string()))
                .wasm_result()?,
        )
        .await
        .map_err(wasm_error)?;
        Ok(WasmNotarizationClientReadOnly(inner_client))
    }

    /// Retrieves the package ID of the used notarization package.
    ///
    /// # Returns
    /// A string representing the package ID.
    #[wasm_bindgen(js_name = packageId)]
    pub fn package_id(&self) -> String {
        self.0.package_id().to_string()
    }

    /// Retrieves the history of notarization package IDs.
    ///
    /// # Returns
    /// An array of strings representing the package history.
    #[wasm_bindgen(js_name = packageHistory)]
    pub fn package_history(&self) -> Vec<String> {
        self.0
            .package_history()
            .into_iter()
            .map(|pkg_id| pkg_id.to_string())
            .collect()
    }

    /// Retrieves the underlying IOTA client used by this client.
    ///
    /// # Returns
    /// The `IotaClient` instance.
    #[wasm_bindgen(js_name = iotaClient)]
    pub fn iota_client(&self) -> WasmIotaClient {
        (*self.0).clone().into_inner()
    }

    /// Retrieves the network identifier associated with this client.
    ///
    /// # Returns
    /// A string representing the network identifier.
    #[wasm_bindgen]
    pub fn network(&self) -> String {
        self.0.network().to_string()
    }

    /// Retrieves the chain ID associated with this client.
    ///
    /// # Returns
    /// A string representing the chain ID.
    #[wasm_bindgen(js_name = chainId)]
    pub fn chain_id(&self) -> String {
        self.0.chain_id().to_string()
    }

    /// Retrieves the [`OnChainNotarization`] of a notarized object.
    ///
    /// This method returns the on-chain notarization object for the given object ID.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of the notarization object.
    ///
    /// # Returns
    /// The [`OnChainNotarization`] object for the given object ID.
    #[wasm_bindgen(js_name = getNotarizationById)]
    pub async fn get_notarization_by_id(&self, notarized_object_id: WasmObjectID) -> Result<WasmOnChainNotarization> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .get_notarization_by_id(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
            .map(Into::into)
    }

    /// Retrieves the timestamp of the last state change for a notarization.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of the notarization object.
    ///
    /// # Returns
    /// The timestamp as `number` value representing the seconds since the Unix epoch.
    #[wasm_bindgen(js_name = lastStateChangeTs)]
    pub async fn last_state_change_ts(&self, notarized_object_id: WasmObjectID) -> Result<u64> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .last_state_change_ts(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Retrieves the creation timestamp for a notarization.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of the notarization object.
    ///
    /// # Returns
    /// The timestamp as `number` value representing the seconds since the Unix epoch.
    #[wasm_bindgen(js_name = createdAtTs)]
    pub async fn created_at_ts(&self, notarized_object_id: WasmObjectID) -> Result<u64> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .created_at_ts(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Retrieves the count of state versions for a notarization.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of a notarization object.
    ///
    /// # Returns
    /// Count as `number` value.
    #[wasm_bindgen(js_name = stateVersionCount)]
    pub async fn state_version_count(&self, notarized_object_id: WasmObjectID) -> Result<u64> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .state_version_count(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Retrieves the description of a notarization.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of a notarization object.
    ///
    /// # Returns
    /// A description string, if existing.
    #[wasm_bindgen]
    pub async fn description(&self, notarized_object_id: WasmObjectID) -> Result<Option<String>> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .description(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Retrieves the updatable metadata of a notarization.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of a notarization object.
    ///
    /// # Returns
    /// A metadata string, if existing.
    #[wasm_bindgen(js_name = updatableMetadata)]
    pub async fn updatable_metadata(&self, notarized_object_id: WasmObjectID) -> Result<Option<String>> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .updatable_metadata(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Retrieves the notarization method of a notarization.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of a notarization object.
    ///
    /// # Returns
    /// The `NotarizationMethod`.
    #[wasm_bindgen(js_name = notarizationMethod)]
    pub async fn notarization_method(&self, notarized_object_id: WasmObjectID) -> Result<WasmNotarizationMethod> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        let notarization_method: WasmNotarizationMethod = self
            .0
            .notarization_method(notarized_object_id)
            .await
            .map_err(wasm_error)?
            .into();
        Ok(notarization_method)
    }

    /// Retrieves the lock metadata of a notarization.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of a notarization object.
    ///
    /// # Returns
    /// The `LockMetadata`, if existing.
    #[wasm_bindgen(js_name = lockMetadata)]
    pub async fn lock_metadata(&self, notarized_object_id: WasmObjectID) -> Result<Option<WasmLockMetadata>> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        let lock_metadata: Option<WasmLockMetadata> = self
            .0
            .lock_metadata(notarized_object_id)
            .await
            .map_err(wasm_error)?
            .map(|meta| meta.into());
        Ok(lock_metadata)
    }

    /// Retrieves the state of a notarization.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of a notarization object.
    ///
    /// # Returns
    /// The notarization `State`.
    #[wasm_bindgen]
    pub async fn state(&self, notarized_object_id: WasmObjectID) -> Result<WasmState> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        let state: WasmState = self.0.state(notarized_object_id).await.map_err(wasm_error)?.into();
        Ok(state)
    }

    /// Checks if updates are locked for a notarization object.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of a notarization object.
    ///
    /// # Returns
    /// A boolean indicating whether updates are locked.
    /// False means that updates are allowed.
    #[wasm_bindgen(js_name = isUpdateLocked)]
    pub async fn is_update_locked(&self, notarized_object_id: WasmObjectID) -> Result<bool> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .is_update_locked(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Checks if destruction is allowed for a notarization object.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of a notarization object.
    ///
    /// # Returns
    /// A boolean indicating whether destruction is allowed.
    /// False means that destroying is not allowed.
    #[wasm_bindgen(js_name = isDestroyAllowed)]
    pub async fn is_destroy_allowed(&self, notarized_object_id: WasmObjectID) -> Result<bool> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .is_destroy_allowed(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Checks if transferring a notarization object is allowed.
    ///
    /// # Arguments
    /// * `notarized_object_id` - The ID of a notarization object.
    ///
    /// # Returns
    /// A boolean indicating whether transfers are locked.
    /// False means that transferring is allowed.
    #[wasm_bindgen(js_name = isTransferLocked)]
    pub async fn is_transfer_locked(&self, notarized_object_id: WasmObjectID) -> Result<bool> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .is_transfer_locked(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }
}
