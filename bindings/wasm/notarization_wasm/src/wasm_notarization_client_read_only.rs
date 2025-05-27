// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::rc::Rc;
use std::str::FromStr;

use notarization::NotarizationClientReadOnly;
use iota_interaction_ts::bindings::WasmIotaClient;
use wasm_bindgen::prelude::*;

use product_common::WasmObjectID;


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
  pub async fn new(iota_client: WasmIotaClient) -> Result<WasmNotarizationClientReadOnly, JsError> {
    let inner_client = NotarizationClientReadOnly::new(iota_client).await?;
    Ok(WasmNotarizationClientReadOnly(inner_client))
  }

  #[wasm_bindgen(js_name = createWithPkgId)]
  pub async fn new_new_with_pkg_id(
    iota_client: WasmIotaClient,
    iota_identity_pkg_id: String,
  ) -> Result<WasmNotarizationClientReadOnly, JsError> {
    let inner_client =
        NotarizationClientReadOnly::new_with_pkg_id(iota_client, ObjectID::from_str(&iota_identity_pkg_id)?).await?;
    Ok(WasmNotarizationClientReadOnly(inner_client))
  }

  #[wasm_bindgen(js_name = packageId)]
  pub fn package_id(&self) -> String {
    self.0.package_id().to_string()
  }

  #[wasm_bindgen(js_name = iotaClient)]
  pub fn iota_client(&self) -> WasmIotaClient {
    (*self.0).clone().into_inner()
  }

  #[wasm_bindgen]
  pub fn network(&self) -> String {
    self.0.network().to_string()
  }
}
