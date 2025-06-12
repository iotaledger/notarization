// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;
use anyhow::anyhow;
use notarization::NotarizationClientReadOnly;
use iota_interaction_ts::bindings::WasmIotaClient;
use iota_interaction_ts::error::{Result, WasmResult, wasm_error};

use iota_interaction::types::base_types::ObjectID;
use product_common::bindings::WasmObjectID;
use product_common::core_client::CoreClientReadOnly;

use wasm_bindgen::prelude::*;

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

  #[wasm_bindgen]
  pub async fn description(&self, notarized_object_id: WasmObjectID) -> Result<Option<String>> {
    let notarized_object_id = ObjectID::from_str(&notarized_object_id)
        .map_err(|e| anyhow!("Could not parse notarized_object_id: {}", e.to_string()))
        .wasm_result()?;
    self.0.description(notarized_object_id)
      .await
      .map_err(wasm_error)
      .wasm_result()
  }
}
