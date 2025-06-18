// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;
use anyhow::anyhow;
use wasm_bindgen::prelude::*;

use notarization::NotarizationClientReadOnly;
use iota_interaction_ts::bindings::WasmIotaClient;
use iota_interaction_ts::error::{Result, WasmResult, wasm_error};
use iota_interaction::types::base_types::ObjectID;
use product_common::bindings::WasmObjectID;
use product_common::bindings::utils::parse_wasm_object_id;
use product_common::core_client::CoreClientReadOnly;

use crate::wasm_types::{WasmNotarizationMethod, WasmLockMetadata, WasmState};

/// A client to interact with Notarization objects on the IOTA ledger.
///
/// Used for read operations, so does not need an account and signing capabilities.
/// If you want to write to the ledger, use {@link NotarizationClient}.
#[derive(Clone)]
#[wasm_bindgen(js_name = NotarizationClientReadOnly)]
pub struct WasmNotarizationClientReadOnly(pub(crate) NotarizationClientReadOnly);

// builder related functions
#[wasm_bindgen(js_class = NotarizationClientReadOnly)]
impl WasmNotarizationClientReadOnly {
  #[wasm_bindgen(js_name = create)]
  pub async fn new(iota_client: WasmIotaClient) -> Result<WasmNotarizationClientReadOnly> {
    let inner_client = NotarizationClientReadOnly::new(iota_client)
        .await
        .map_err(wasm_error)?;
    Ok(WasmNotarizationClientReadOnly(inner_client))
  }

  #[wasm_bindgen(js_name = createWithPkgId)]
  pub async fn new_new_with_pkg_id(
    iota_client: WasmIotaClient,
    iota_notarization_pkg_id: String,
  ) -> Result<WasmNotarizationClientReadOnly> {
    let inner_client =
        NotarizationClientReadOnly::new_with_pkg_id(
          iota_client,
          ObjectID::from_str(&iota_notarization_pkg_id)
              .map_err(|e| anyhow!("Could not parse iota_notarization_pkg_id: {}", e.to_string()))
              .wasm_result()?
        )
        .await
        .map_err(wasm_error)?;
    Ok(WasmNotarizationClientReadOnly(inner_client))
  }

  #[wasm_bindgen(js_name = packageId)]
  pub fn package_id(&self) -> String {
    self.0.package_id().to_string()
  }

  #[wasm_bindgen(js_name = packageHistory)]
  pub fn package_history(&self) -> Vec<String> {
    self.0.package_history()
        .into_iter()
        .map(|pkg_id| pkg_id.to_string())
        .collect()
  }

  #[wasm_bindgen(js_name = iotaClient)]
  pub fn iota_client(&self) -> WasmIotaClient {
    (*self.0).clone().into_inner()
  }

  #[wasm_bindgen]
  pub fn network(&self) -> String {
    self.0.network().to_string()
  }

  #[wasm_bindgen(js_name = chainId)]
  pub fn chain_id(&self) -> String {
    self.0.chain_id().to_string()
  }

  #[wasm_bindgen(js_name = lastStateChangeTs)]
  pub async fn last_state_change_ts(&self, notarized_object_id: WasmObjectID) -> Result<u64> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    self.0.last_state_change_ts(notarized_object_id)
        .await
        .map_err(wasm_error)
        .wasm_result()
  }

  #[wasm_bindgen(js_name = createdAtTs)]
  pub async fn created_at_ts(&self, notarized_object_id: WasmObjectID) -> Result<u64> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    self.0.created_at_ts(notarized_object_id)
        .await
        .map_err(wasm_error)
        .wasm_result()
  }

  #[wasm_bindgen(js_name = stateVersionCount)]
  pub async fn state_version_count(&self, notarized_object_id: WasmObjectID) -> Result<u64> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    self.0.state_version_count(notarized_object_id)
        .await
        .map_err(wasm_error)
        .wasm_result()
  }

  #[wasm_bindgen]
  pub async fn description(&self, notarized_object_id: WasmObjectID) -> Result<Option<String>> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    self.0.description(notarized_object_id)
      .await
      .map_err(wasm_error)
      .wasm_result()
  }

  #[wasm_bindgen(js_name = updatableMetadata)]
  pub async fn updatable_metadata(&self, notarized_object_id: WasmObjectID) -> Result<Option<String>> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    self.0.updatable_metadata(notarized_object_id)
        .await
        .map_err(wasm_error)
        .wasm_result()
  }

  #[wasm_bindgen(js_name = notarizationMethod)]
  pub async fn notarization_method(&self, notarized_object_id: WasmObjectID) -> Result<WasmNotarizationMethod> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    let notarization_method: WasmNotarizationMethod = self.0.notarization_method(notarized_object_id)
        .await
        .map_err(wasm_error)?
        .into();
    Ok(notarization_method)
  }

  #[wasm_bindgen(js_name = lockMetadata)]
  pub async fn lock_metadata(&self, notarized_object_id: WasmObjectID) -> Result<Option<WasmLockMetadata>> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    let lock_metadata: Option<WasmLockMetadata> = self.0.lock_metadata(notarized_object_id)
        .await
        .map_err(wasm_error)?
        .map(|meta| meta.into());
    Ok(lock_metadata)
  }

  #[wasm_bindgen]
  pub async fn state(&self, notarized_object_id: WasmObjectID) -> Result<WasmState> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    let state: WasmState = self.0.state(notarized_object_id)
        .await
        .map_err(wasm_error)?
        .into();
    Ok(state)
  }

  #[wasm_bindgen(js_name = isUpdateLocked)]
  pub async fn is_update_locked(&self, notarized_object_id: WasmObjectID) -> Result<bool> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    self.0.is_update_locked(notarized_object_id)
        .await
        .map_err(wasm_error)
        .wasm_result()
  }

  #[wasm_bindgen(js_name = isDestroyAllowed)]
  pub async fn is_destroy_allowed(&self, notarized_object_id: WasmObjectID) -> Result<bool> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    self.0.is_destroy_allowed(notarized_object_id)
        .await
        .map_err(wasm_error)
        .wasm_result()
  }

  #[wasm_bindgen(js_name = isTransferLocked)]
  pub async fn is_transfer_locked(&self, notarized_object_id: WasmObjectID) -> Result<bool> {
    let notarized_object_id = parse_wasm_object_id(&notarized_object_id)?;
    self.0.is_transfer_locked(notarized_object_id)
        .await
        .map_err(wasm_error)
        .wasm_result()
  }
}
