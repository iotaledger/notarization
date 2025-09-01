// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction_ts::bindings::{WasmIotaClient, WasmPublicKey, WasmTransactionSigner};
use iota_interaction_ts::wasm_error::{Result, WasmResult};
use notarization::NotarizationClient;
use product_common::bindings::transaction::WasmTransactionBuilder;
use product_common::bindings::utils::{into_transaction_builder, parse_wasm_iota_address, parse_wasm_object_id};
use product_common::bindings::{WasmIotaAddress, WasmObjectID};
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use wasm_bindgen::prelude::*;

use crate::wasm_notarization::{
    WasmDestroyNotarization, WasmTransferNotarization, WasmUpdateMetadata, WasmUpdateState,
};
use crate::wasm_notarization_builder::{WasmNotarizationBuilderDynamic, WasmNotarizationBuilderLocked};
use crate::wasm_notarization_client_read_only::WasmNotarizationClientReadOnly;
use crate::wasm_types::WasmState;

/// A client to interact with Notarization objects on the IOTA ledger.
///
/// This client is used for read and write operations. For read-only capabilities,
/// you can use {@link NotarizationClientReadOnly}, which does not require an account or signing capabilities.
#[derive(Clone)]
#[wasm_bindgen(js_name = NotarizationClient)]
pub struct WasmNotarizationClient(pub(crate) NotarizationClient<WasmTransactionSigner>);

// builder related functions
#[wasm_bindgen(js_class = NotarizationClient)]
impl WasmNotarizationClient {
    /// Creates a new notarization client.
    ///
    /// # Arguments
    /// * `client` - A read-only notarization client.
    /// * `signer` - A transaction signer for signing operations.
    ///
    /// # Returns
    /// A `TransactionBuilder` to build and execute the transaction.
    #[wasm_bindgen(js_name = create)]
    pub async fn new(
        client: WasmNotarizationClientReadOnly,
        signer: WasmTransactionSigner,
    ) -> Result<WasmNotarizationClient> {
        let inner_client = NotarizationClient::new(client.0, signer).await.wasm_result()?;
        Ok(WasmNotarizationClient(inner_client))
    }

    /// Retrieves the sender's public key.
    ///
    /// # Returns
    /// The sender's public key as `PublicKey`.
    #[wasm_bindgen(js_name = senderPublicKey)]
    pub fn sender_public_key(&self) -> Result<WasmPublicKey> {
        self.0.sender_public_key().try_into()
    }

    /// Retrieves the sender's address.
    ///
    /// # Returns
    /// The sender's address as an `IotaAddress`.
    #[wasm_bindgen(js_name = senderAddress)]
    pub fn sender_address(&self) -> WasmIotaAddress {
        self.0.sender_address().to_string()
    }

    /// Retrieves the network identifier.
    ///
    /// # Returns
    /// The network identifier as a `string`.
    #[wasm_bindgen(js_name = network)]
    pub fn network(&self) -> String {
        self.0.network().to_string()
    }

    /// Retrieves the package ID.
    ///
    /// # Returns
    /// The package ID as a `string`.
    #[wasm_bindgen(js_name = packageId)]
    pub fn package_id(&self) -> String {
        self.0.package_id().to_string()
    }

    /// Retrieves the package history.
    ///
    /// # Returns
    /// An `Array<string>` containing the package history.
    #[wasm_bindgen(js_name = packageHistory)]
    pub fn package_history(&self) -> Vec<String> {
        self.0
            .package_history()
            .into_iter()
            .map(|pkg_id| pkg_id.to_string())
            .collect()
    }

    /// Retrieves the IOTA client instance.
    ///
    /// # Returns
    /// The `IotaClient` instance.
    #[wasm_bindgen(js_name = iotaClient)]
    pub fn iota_client(&self) -> WasmIotaClient {
        (**self.0).clone().into_inner()
    }

    /// Retrieves the transaction signer.
    ///
    /// # Returns
    /// The `TransactionSigner` instance.
    #[wasm_bindgen]
    pub fn signer(&self) -> WasmTransactionSigner {
        self.0.signer().clone()
    }

    /// Retrieves a read-only version of the notarization client.
    ///
    /// # Returns
    /// A `NotarizationClientReadOnly` instance.
    #[wasm_bindgen(js_name = readOnly)]
    pub fn read_only(&self) -> WasmNotarizationClientReadOnly {
        WasmNotarizationClientReadOnly((*self.0).clone())
    }

    /// Creates a notarization builder which can be used to create a dynamic notarization.
    ///
    /// # Returns
    /// A `NotarizationBuilderDynamic` instance.
    #[wasm_bindgen(js_name = createDynamic)]
    pub fn create_dynamic(&self) -> WasmNotarizationBuilderDynamic {
        WasmNotarizationBuilderDynamic(self.0.create_dynamic_notarization())
    }

    /// Creates a notarization builder which can be used to create a locked notarization.
    ///
    /// # Returns
    /// A `NotarizationBuilderLocked` instance.
    #[wasm_bindgen(js_name = createLocked)]
    pub fn create_locked(&self) -> WasmNotarizationBuilderLocked {
        WasmNotarizationBuilderLocked(self.0.create_locked_notarization())
    }

    /// Creates a transaction to update the `state` of a notarization.
    ///
    /// **Important**: The `state` can only  be updated depending on the used `NotarizationMethod`:
    /// - Dynamic: Can be updated anytime after notarization creation
    /// - Locked: Immutable after notarization creation
    ///
    /// Using this function will:
    /// - set the `state` to the `new_state`
    /// - increase the `stateVersionCount` by 1
    /// - set the `lastStateChangeAt` timestamp to the current clock  timestamp in milliseconds
    /// - emits a `NotarizationUpdated` Move event in case of success
    /// - fail if the notarization uses `NotarizationMethod` `Locked`
    ///
    /// # Arguments
    /// * `new_state` - The new state to replace the current one.
    /// * `notarization_id` - The ID of the notarization object.
    ///
    /// # Returns
    /// A `TransactionBuilder` to build and execute the transaction.
    #[wasm_bindgen(js_name = updateState)]
    pub fn update_state(&self, new_state: WasmState, notarization_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
        let notarization_id = parse_wasm_object_id(&notarization_id)?;
        let tx = self.0.update_state(new_state.0, notarization_id).into_inner();
        Ok(into_transaction_builder(WasmUpdateState(tx)))
    }

    /// Creates a transaction to update the metadata of a notarization.
    ///
    /// **Important**: The `updatableMetadata` can only be updated depending on the used
    /// `NotarizationMethod`:
    /// - Dynamic: Can be updated anytime after notarization creation
    /// - Locked: Immutable after notarization creation
    ///
    /// NOTE:
    /// - does not affect the `stateVersionCount` or the `lastStateChangeAt` timestamp
    /// - will fail if the notarization uses the `NotarizationMethod::Locked`
    /// - Only the `updatableMetadata` can be changed; the `immutableMetadata::description`
    ///   remains fixed
    /// # Arguments
    /// * `metadata` - The new metadata to update (optional).
    /// * `notarization_id` - The ID of the notarization object.
    ///
    /// # Returns
    /// A `TransactionBuilder` to build and execute the transaction.
    #[wasm_bindgen(js_name = updateMetadata)]
    pub fn update_metadata(
        &self,
        metadata: Option<String>,
        notarization_id: WasmObjectID,
    ) -> Result<WasmTransactionBuilder> {
        let notarization_id = parse_wasm_object_id(&notarization_id)?;
        let tx = self.0.update_metadata(metadata, notarization_id).into_inner();
        Ok(into_transaction_builder(WasmUpdateMetadata(tx)))
    }

    /// Creates a transaction to destroy a notarization object on the ledger.
    ///
    /// # Arguments
    /// * `notarization_id` - The ID of the notarization object to destroy.
    ///
    /// # Returns
    /// A `TransactionBuilder` to build and execute the transaction.
    #[wasm_bindgen(js_name = destroy)]
    pub fn destroy_notarization(&self, notarization_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
        let notarization_id = parse_wasm_object_id(&notarization_id)?;
        let tx = self.0.destroy(notarization_id).into_inner();
        Ok(into_transaction_builder(WasmDestroyNotarization(tx)))
    }

    /// Creates a transaction to transfer a notarization object to a new owner.
    ///
    /// # Arguments
    /// * `notarization_id` - The ID of the notarization object to transfer.
    /// * `recipient` - The recipient's IOTA address.
    ///
    /// # Returns
    /// A `TransactionBuilder` to build and execute the transaction.
    #[wasm_bindgen(js_name = transferNotarization)]
    pub fn transfer_notarization(
        &self,
        notarization_id: WasmObjectID,
        recipient: WasmIotaAddress,
    ) -> Result<WasmTransactionBuilder> {
        let notarization_id = parse_wasm_object_id(&notarization_id)?;
        let recipient_address = parse_wasm_iota_address(&recipient)?;
        let tx = self
            .0
            .transfer_notarization(notarization_id, recipient_address)
            .into_inner();
        Ok(into_transaction_builder(WasmTransferNotarization(tx)))
    }
}
