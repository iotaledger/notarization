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

/// Read-write client for creating and modifying notarizations on the IOTA
/// ledger.
///
/// @remarks
/// Wraps a {@link NotarizationClientReadOnly} together with a transaction
/// signer. Use the builder methods ({@link NotarizationClient.createDynamic},
/// {@link NotarizationClient.createLocked}) to create new notarizations and
/// the mutation methods ({@link NotarizationClient.updateState},
/// {@link NotarizationClient.updateMetadata},
/// {@link NotarizationClient.destroy},
/// {@link NotarizationClient.transferNotarization}) to operate on existing
/// ones. For pure read access, prefer {@link NotarizationClientReadOnly}.
#[derive(Clone)]
#[wasm_bindgen(js_name = NotarizationClient)]
pub struct WasmNotarizationClient(pub(crate) NotarizationClient<WasmTransactionSigner>);

#[wasm_bindgen(js_class = NotarizationClient)]
impl WasmNotarizationClient {
    /// Constructs a read-write client by attaching a signer to a read-only
    /// client.
    ///
    /// @param client - A {@link NotarizationClientReadOnly} connected to the
    /// target network.
    /// @param signer - A {@link TransactionSigner} responsible for signing
    /// outgoing transactions.
    ///
    /// @returns A connected {@link NotarizationClient}.
    ///
    /// @throws When the signer's public key cannot be retrieved.
    #[wasm_bindgen(js_name = create)]
    pub async fn new(
        client: WasmNotarizationClientReadOnly,
        signer: WasmTransactionSigner,
    ) -> Result<WasmNotarizationClient> {
        let inner_client = NotarizationClient::new(client.0, signer).await.wasm_result()?;
        Ok(WasmNotarizationClient(inner_client))
    }

    /// The signer's public key.
    ///
    /// @throws When the signer fails to provide its public key.
    #[wasm_bindgen(js_name = senderPublicKey)]
    pub fn sender_public_key(&self) -> Result<WasmPublicKey> {
        self.0.sender_public_key().try_into()
    }

    /// The IOTA address transactions will be sent from.
    #[wasm_bindgen(js_name = senderAddress)]
    pub fn sender_address(&self) -> WasmIotaAddress {
        self.0.sender_address().to_string()
    }

    /// The network identifier this client is connected to.
    #[wasm_bindgen(js_name = network)]
    pub fn network(&self) -> String {
        self.0.network().to_string()
    }

    /// The notarization package ID this client is using.
    #[wasm_bindgen(js_name = packageId)]
    pub fn package_id(&self) -> String {
        self.0.package_id().to_string()
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

    /// The TF-Components package ID for product_common compatibility.
    ///
    /// Notarization uses the package-local `timelock` module, so this is
    /// always `undefined`.
    #[wasm_bindgen(js_name = tfComponentsPackageId)]
    pub fn tf_components_package_id(&self) -> Option<String> {
        None
    }

    /// The underlying IOTA client used for ledger queries.
    #[wasm_bindgen(js_name = iotaClient)]
    pub fn iota_client(&self) -> WasmIotaClient {
        (**self.0).clone().into_inner()
    }

    /// The transaction signer attached to this client.
    #[wasm_bindgen]
    pub fn signer(&self) -> WasmTransactionSigner {
        self.0.signer().clone()
    }

    /// Returns a read-only view of this client.
    ///
    /// @returns A {@link NotarizationClientReadOnly} sharing the same network
    /// connection.
    #[wasm_bindgen(js_name = readOnly)]
    pub fn read_only(&self) -> WasmNotarizationClientReadOnly {
        WasmNotarizationClientReadOnly((*self.0).clone())
    }

    /// Starts building a Dynamic-Notarization.
    ///
    /// @remarks
    /// On execution the resulting transaction transfers the new notarization
    /// object to the sender.
    ///
    /// @returns A fresh {@link NotarizationBuilderDynamic}.
    ///
    /// Emits a `DynamicNotarizationCreated` event on success.
    #[wasm_bindgen(js_name = createDynamic)]
    pub fn create_dynamic(&self) -> WasmNotarizationBuilderDynamic {
        WasmNotarizationBuilderDynamic(self.0.create_dynamic_notarization())
    }

    /// Starts building a Locked-Notarization.
    ///
    /// @remarks
    /// On execution the resulting transaction transfers the new notarization
    /// object to the sender.
    ///
    /// @returns A fresh {@link NotarizationBuilderLocked}.
    ///
    /// Emits a `LockedNotarizationCreated` event on success.
    #[wasm_bindgen(js_name = createLocked)]
    pub fn create_locked(&self) -> WasmNotarizationBuilderLocked {
        WasmNotarizationBuilderLocked(self.0.create_locked_notarization())
    }

    /// Builds a transaction that replaces the state of a notarization.
    ///
    /// @remarks
    /// On success the on-chain transaction replaces `state` with `newState`,
    /// increments `stateVersionCount` by `1`, and refreshes
    /// `lastStateChangeAt` to the on-chain clock (in milliseconds since the
    /// Unix epoch).
    ///
    /// Behaviour depends on the Notarization Method:
    /// * `Dynamic`: always permitted — the underlying `updateLock` is fixed to {@link TimeLockType.None}.
    /// * `Locked`: always aborts on-chain, because the underlying `updateLock` is pinned to {@link
    ///   TimeLockType.UntilDestroyed}.
    ///
    /// @param newState - The replacement {@link State}.
    /// @param notarizationId - The notarization object's ID.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link UpdateState} transaction.
    ///
    /// @throws When the ID is malformed.
    ///
    /// Emits a `NotarizationUpdated` event on success.
    #[wasm_bindgen(js_name = updateState)]
    pub fn update_state(&self, new_state: WasmState, notarization_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
        let notarization_id = parse_wasm_object_id(&notarization_id)?;
        let tx = self.0.update_state(new_state.0, notarization_id).into_inner();
        Ok(into_transaction_builder(WasmUpdateState(tx)))
    }

    /// Builds a transaction that replaces the updatable metadata of a
    /// notarization.
    ///
    /// @remarks
    /// Does not affect the `state`, `stateVersionCount`,
    /// `lastStateChangeAt`, or the immutable description.
    ///
    /// Behaviour depends on the Notarization Method:
    /// * `Dynamic`: always permitted — the underlying `updateLock` is fixed to {@link TimeLockType.None}.
    /// * `Locked`: always aborts on-chain, because the underlying `updateLock` is pinned to {@link
    ///   TimeLockType.UntilDestroyed}.
    ///
    /// @param metadata - The replacement metadata, or `null` to clear it.
    /// @param notarizationId - The notarization object's ID.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link UpdateMetadata} transaction.
    ///
    /// @throws When the ID is malformed.
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

    /// Builds a transaction that destroys a notarization permanently and
    /// releases its object ID.
    ///
    /// @remarks
    /// All package-local {@link TimeLock}s of the attached {@link LockMetadata}
    /// are destroyed in the process. The notarization must currently be
    /// destroy-allowed (see
    /// {@link NotarizationClientReadOnly.isDestroyAllowed}); otherwise the
    /// on-chain transaction aborts.
    ///
    /// @param notarizationId - The notarization object's ID.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link DestroyNotarization} transaction.
    ///
    /// @throws When the ID is malformed.
    ///
    /// Emits a `NotarizationDestroyed` event on success.
    #[wasm_bindgen(js_name = destroy)]
    pub fn destroy_notarization(&self, notarization_id: WasmObjectID) -> Result<WasmTransactionBuilder> {
        let notarization_id = parse_wasm_object_id(&notarization_id)?;
        let tx = self.0.destroy(notarization_id).into_inner();
        Ok(into_transaction_builder(WasmDestroyNotarization(tx)))
    }

    /// Builds a transaction that transfers ownership of a notarization to
    /// another address.
    ///
    /// @remarks
    /// Permitted only when the notarization has no {@link LockMetadata} or
    /// when its `transferLock` is not currently active.
    ///
    /// Behaviour depends on the Notarization Method:
    /// * `Dynamic`: on success the notarization is transferred to `recipient`. Submitting while the configured
    ///   `transferLock` is currently engaged aborts on-chain.
    /// * `Locked`: always aborts on-chain — Locked-Notarizations have their `transferLock` pinned to {@link
    ///   TimeLockType.UntilDestroyed} and are therefore non-transferable.
    ///
    /// @param notarizationId - The notarization object's ID.
    /// @param recipient - The new owner's IOTA address.
    ///
    /// @returns A {@link TransactionBuilder} wrapping the
    /// {@link TransferNotarization} transaction.
    ///
    /// @throws When the ID or address is malformed.
    ///
    /// Emits a `DynamicNotarizationTransferred` event on success.
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
