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

/// Read-only client for inspecting notarization objects on the IOTA ledger.
///
/// @remarks
/// This client never signs or submits transactions; use {@link NotarizationClient}
/// for write operations. All accessor methods take a notarization object ID
/// and return the corresponding on-chain value.
#[derive(Clone)]
#[wasm_bindgen(js_name = NotarizationClientReadOnly)]
pub struct WasmNotarizationClientReadOnly(pub(crate) NotarizationClientReadOnly);

#[wasm_bindgen(js_class = NotarizationClientReadOnly)]
impl WasmNotarizationClientReadOnly {
    /// Constructs a read-only client and resolves the notarization package
    /// for the network the given IOTA client is connected to.
    ///
    /// @param iotaClient - An IOTA client connected to the target network.
    ///
    /// @returns A connected {@link NotarizationClientReadOnly}.
    ///
    /// @throws When the network cannot be queried or no notarization package
    /// is available for it.
    #[wasm_bindgen(js_name = create)]
    pub async fn new(iota_client: WasmIotaClient) -> Result<WasmNotarizationClientReadOnly> {
        let inner_client = NotarizationClientReadOnly::new(iota_client).await.map_err(wasm_error)?;
        Ok(WasmNotarizationClientReadOnly(inner_client))
    }

    /// Constructs a read-only client pinned to a specific notarization
    /// package ID.
    ///
    /// @remarks
    /// Use this when you need to interact with a particular package version,
    /// e.g. for replaying historical state, instead of letting the client
    /// resolve the latest package on the network.
    ///
    /// @param iotaClient - An IOTA client connected to the target network.
    /// @param iotaNotarizationPkgId - The notarization package ID to pin to.
    ///
    /// @returns A connected {@link NotarizationClientReadOnly}.
    ///
    /// @throws When the package ID cannot be parsed or the network cannot be
    /// queried.
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

    /// The notarization package ID this client is using.
    #[wasm_bindgen(js_name = packageId)]
    pub fn package_id(&self) -> String {
        self.0.package_id().to_string()
    }

    /// Returns the `tf_components` package ID currently in use.
    ///
    /// @returns Stringified object ID of the resolved `tf_components` package.
    #[wasm_bindgen(js_name = tfComponentsPackageId)]
    pub fn tf_components_package_id(&self) -> String {
        self.0.tf_components_package_id().unwrap_or(ObjectID::ZERO).to_string()
    }

    /// The full history of notarization package IDs known on this network,
    /// most recent first.
    #[wasm_bindgen(js_name = packageHistory)]
    pub fn package_history(&self) -> Vec<String> {
        self.0
            .package_history()
            .into_iter()
            .map(|pkg_id| pkg_id.to_string())
            .collect()
    }

    /// The underlying IOTA client used for ledger queries.
    #[wasm_bindgen(js_name = iotaClient)]
    pub fn iota_client(&self) -> WasmIotaClient {
        (*self.0).clone().into_inner()
    }

    /// The network identifier (e.g. `"mainnet"`, `"testnet"`) this client is
    /// connected to.
    #[wasm_bindgen]
    pub fn network(&self) -> String {
        self.0.network().to_string()
    }

    /// The chain ID this client is connected to.
    #[wasm_bindgen(js_name = chainId)]
    pub fn chain_id(&self) -> String {
        self.0.chain_id().to_string()
    }

    /// Fetches the on-chain representation of a notarization.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns The {@link OnChainNotarization} for the given ID.
    ///
    /// @throws When the ID is malformed or no notarization with that ID
    /// exists on the connected network.
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

    /// Fetches the timestamp of the most recent state change.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns Milliseconds since the Unix epoch.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
    #[wasm_bindgen(js_name = lastStateChangeTs)]
    pub async fn last_state_change_ts(&self, notarized_object_id: WasmObjectID) -> Result<u64> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .last_state_change_ts(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Fetches the creation timestamp.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns Milliseconds since the Unix epoch.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
    #[wasm_bindgen(js_name = createdAtTs)]
    pub async fn created_at_ts(&self, notarized_object_id: WasmObjectID) -> Result<u64> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .created_at_ts(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Fetches the number of state versions a notarization has gone through.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns The count, where `0` means the state has not been updated
    /// since creation.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
    #[wasm_bindgen(js_name = stateVersionCount)]
    pub async fn state_version_count(&self, notarized_object_id: WasmObjectID) -> Result<u64> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .state_version_count(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Fetches the immutable description set at creation, if any.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns The description string, or `null` when none was set.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
    #[wasm_bindgen]
    pub async fn description(&self, notarized_object_id: WasmObjectID) -> Result<Option<String>> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .description(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Fetches the current updatable metadata, if any.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns The metadata string, or `null` when none is set.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
    #[wasm_bindgen(js_name = updatableMetadata)]
    pub async fn updatable_metadata(&self, notarized_object_id: WasmObjectID) -> Result<Option<String>> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .updatable_metadata(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Fetches the {@link NotarizationMethod} of a notarization.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns The notarization's {@link NotarizationMethod}.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
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

    /// Fetches the {@link LockMetadata} attached at creation, if any.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns The {@link LockMetadata}, or `null` when none is attached.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
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

    /// Fetches the current {@link State} of a notarization.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns The current {@link State}.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
    #[wasm_bindgen]
    pub async fn state(&self, notarized_object_id: WasmObjectID) -> Result<WasmState> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        let state: WasmState = self.0.state(notarized_object_id).await.map_err(wasm_error)?.into();
        Ok(state)
    }

    /// Checks whether state updates are currently locked.
    ///
    /// @remarks
    /// Result depends on the Notarization Method:
    /// * `Dynamic`: always `false`.
    /// * `Locked`: `true` while the configured `updateLock` is engaged.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns `true` if state updates are currently rejected, `false`
    /// otherwise.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
    #[wasm_bindgen(js_name = isUpdateLocked)]
    pub async fn is_update_locked(&self, notarized_object_id: WasmObjectID) -> Result<bool> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .is_update_locked(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Checks whether the notarization can currently be destroyed.
    ///
    /// @remarks
    /// Behaviour depends on the Notarization Method:
    /// * `Dynamic`: destruction is gated only on the `transferLock`. The notarization is destroy-allowed unless
    ///   `transferLock` is currently `UnlockAt`-locked.
    /// * `Locked`: destruction is gated on `updateLock`, `deleteLock`, and `transferLock`. The notarization is
    ///   destroy-allowed only when none of them is currently `UnlockAt`-locked.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns `true` if {@link NotarizationClient.destroy} would currently
    /// succeed, `false` otherwise.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
    #[wasm_bindgen(js_name = isDestroyAllowed)]
    pub async fn is_destroy_allowed(&self, notarized_object_id: WasmObjectID) -> Result<bool> {
        let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
        self.0
            .is_destroy_allowed(notarized_object_id)
            .await
            .map_err(wasm_error)
            .wasm_result()
    }

    /// Checks whether ownership transfers are currently locked.
    ///
    /// @remarks
    /// Result depends on the Notarization Method:
    /// * `Dynamic`: `true` when the configured `transferLock` is engaged.
    /// * `Locked`: always `true` — Locked-Notarizations are non-transferable by design.
    ///
    /// @param notarizedObjectId - The notarization object's ID.
    ///
    /// @returns `true` if {@link NotarizationClient.transferNotarization}
    /// would currently abort, `false` otherwise.
    ///
    /// @throws When the ID is malformed or the object cannot be fetched.
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
