// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::prelude::*;

use product_common::bindings::WasmIotaAddress;
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::WasmObjectID;
use product_common::bindings::utils::{into_transaction_builder, parse_wasm_iota_address, parse_wasm_object_id};
use product_common::core_client::CoreClient;
use product_common::core_client::CoreClientReadOnly;
use iota_interaction_ts::bindings::{WasmTransactionSigner, WasmPublicKey, WasmIotaClient};
use iota_interaction_ts::error::{Result, WasmResult};

use notarization::NotarizationClient;

use crate::wasm_notarization_client_read_only::WasmNotarizationClientReadOnly;
use crate::wasm_notarization_builder::WasmNotarizationBuilderDynamic;
use crate::wasm_notarization_builder::WasmNotarizationBuilderLocked;
use crate::wasm_types::WasmState;
use crate::wasm_notarization::{WasmUpdateMetadata, WasmUpdateState, WasmDestroyNotarization};
use crate::wasm_notarization::WasmTransferNotarization;

/// A client to interact with identities on the IOTA chain.
///
/// Used for read and write operations. If you just want read capabilities,
/// you can also use {@link NotarizationClientReadOnly}, which does not need an account and signing capabilities.
#[derive(Clone)]
#[wasm_bindgen(js_name = NotarizationClient)]
pub struct WasmNotarizationClient(pub(crate) NotarizationClient<WasmTransactionSigner>);

// builder related functions
#[wasm_bindgen(js_class = NotarizationClient)]
impl WasmNotarizationClient {
  #[wasm_bindgen(js_name = create)]
  pub async fn new(client: WasmNotarizationClientReadOnly, signer: WasmTransactionSigner) -> Result<WasmNotarizationClient> {
    let inner_client = NotarizationClient::new(client.0, signer).await.wasm_result()?;
    Ok(WasmNotarizationClient(inner_client))
  }

  #[wasm_bindgen(js_name = senderPublicKey)]
  pub fn sender_public_key(&self) -> Result<WasmPublicKey> {
    self.0.sender_public_key().try_into()
  }

  #[wasm_bindgen(js_name = senderAddress)]
  pub fn sender_address(&self) -> WasmIotaAddress {
    self.0.sender_address().to_string()
  }

  #[wasm_bindgen(js_name = network)]
  pub fn network(&self) -> String {
    self.0.network().to_string()
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
    (**self.0).clone().into_inner()
  }

  #[wasm_bindgen]
  pub fn signer(&self) -> WasmTransactionSigner {
    self.0.signer().clone()
  }

  #[wasm_bindgen(js_name = readOnly)]
  pub fn read_only(&self) -> WasmNotarizationClientReadOnly {
    WasmNotarizationClientReadOnly((*self.0).clone())
  }

  #[wasm_bindgen(js_name = createDynamic)]
  pub fn create_dynamic(&self) -> WasmNotarizationBuilderDynamic {
    WasmNotarizationBuilderDynamic(self.0.create_dynamic_notarization())
  }

  #[wasm_bindgen(js_name = createLocked)]
  pub fn create_locked(&self) -> WasmNotarizationBuilderLocked {
    WasmNotarizationBuilderLocked(self.0.create_locked_notarization())
  }

  /// Creates a transaction that updates the state of a notarization
  #[wasm_bindgen(js_name = updateState)]
  pub fn update_state(&self, state: WasmState, object_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
    let obj_id = parse_wasm_object_id(&object_id)?;
    let tx = self.0.update_state(state.0, obj_id).into_inner();
    Ok(into_transaction_builder(WasmUpdateState(tx)))
  }

  /// Creates a transaction that updates the metadata of a notarization
  #[wasm_bindgen(js_name = updateMetadata)]
  pub fn update_metadata(&self, metadata: Option<String>, object_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
    let obj_id = parse_wasm_object_id(&object_id)?;
    let tx = self.0.update_metadata(metadata, obj_id).into_inner();
    Ok(into_transaction_builder(WasmUpdateMetadata(tx)))
  }

  /// Creates a transaction that destroys a notarization object on the ledger
  #[wasm_bindgen(js_name = destroy)]
  pub fn destroy_notarization(&self, object_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
    let obj_id = parse_wasm_object_id(&object_id)?;
    let tx = self.0.destroy(obj_id).into_inner();
    Ok(into_transaction_builder(WasmDestroyNotarization(tx)))
  }

  /// Creates a transaction that transfers a notarization object to a new owner
  #[wasm_bindgen(js_name = transferNotarization)]
  pub fn transfer_notarization(
    &self,
    object_id: WasmObjectID,
    recipient: WasmIotaAddress,
  ) -> Result<WasmTransactionBuilder> {
    let obj_id = parse_wasm_object_id(&object_id)?;
    let recipient_address = parse_wasm_iota_address(&recipient)?;
    let tx = self.0.transfer_notarization(obj_id, recipient_address).into_inner();
    Ok(into_transaction_builder(WasmTransferNotarization(tx)))
  }
}
