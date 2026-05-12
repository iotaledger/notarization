// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_interaction_ts::bindings::{WasmIotaTransactionBlockEffects, WasmIotaTransactionBlockEvents};
use iota_interaction_ts::core_client::WasmCoreClientReadOnly;
use iota_interaction_ts::wasm_error::Result;
use notarization::core::builder::{Dynamic, Locked};
use notarization::core::transactions::{
    CreateNotarization, DestroyNotarization, TransferNotarization, UpdateMetadata, UpdateState,
};
use notarization::core::types::irl_integration::NotarizationResourceBuilder;
use notarization::core::types::OnChainNotarization;
use product_common::bindings::utils::{
    apply_with_events, build_programmable_transaction, parse_wasm_iota_address, parse_wasm_object_id,
};
use product_common::bindings::{WasmIotaAddress, WasmObjectID};
use product_common::network_name::NetworkName;
use wasm_bindgen::prelude::*;

use crate::wasm_notarization_builder::{WasmNotarizationBuilderDynamic, WasmNotarizationBuilderLocked};
use crate::wasm_types::{WasmEmpty, WasmImmutableMetadata, WasmNotarizationMethod, WasmState};

/// The on-chain representation of a notarization.
///
/// @remarks
/// Stores user-defined data together with immutable provenance, optional
/// updatable metadata, and lock metadata that governs whether the object can
/// be updated, transferred, or destroyed. The selected
/// {@link NotarizationMethod} determines which mutations are allowed after
/// creation.
///
/// Returned by {@link NotarizationClientReadOnly.getNotarizationById} and by
/// the executed transaction. Exposes the notarization's identity, current
/// state, immutable and updatable metadata, the {@link NotarizationMethod},
/// and the current owner.
#[wasm_bindgen(js_name = OnChainNotarization, inspectable)]
#[derive(Clone)]
pub struct WasmOnChainNotarization(pub(crate) OnChainNotarization);

#[wasm_bindgen(js_class = OnChainNotarization)]
impl WasmOnChainNotarization {
    pub(crate) fn new(notarization: OnChainNotarization) -> Self {
        Self(notarization)
    }

    /// The notarization's object ID, as a hexadecimal string.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.id.bytes.to_hex()
    }

    /// The current {@link State} of the notarization.
    ///
    /// @remarks
    /// The `state` of a notarization contains the notarized data and metadata
    /// associated with the current version of the `state`.
    ///
    /// Mutability depends on the Notarization Method:
    /// * `Dynamic`: the state can be replaced after creation via
    ///   {@link NotarizationClient.updateState}.
    /// * `Locked`: the state is fixed at creation and cannot be replaced.
    #[wasm_bindgen(getter)]
    pub fn state(&self) -> WasmState {
        WasmState(self.0.state.clone())
    }

    /// The fixed-at-creation {@link ImmutableMetadata}.
    ///
    /// @remarks
    /// Provides immutable information, assertions, and guarantees for third
    /// parties: it is created automatically at notarization creation and
    /// cannot be changed afterwards.
    #[wasm_bindgen(js_name = immutableMetadata, getter)]
    pub fn immutable_metadata(&self) -> WasmImmutableMetadata {
        WasmImmutableMetadata(self.0.immutable_metadata.clone())
    }

    /// The current updatable metadata, if any.
    ///
    /// @remarks
    /// Provides context or additional information for third parties.
    ///
    /// Mutability depends on the Notarization Method:
    /// * `Dynamic`: the updatable metadata can be replaced after creation
    ///   via {@link NotarizationClient.updateMetadata}.
    /// * `Locked`: the value is fixed at creation and cannot be replaced.
    ///
    /// `updatableMetadata` is independent of {@link OnChainNotarization.state}:
    /// replacing it does not increment the `stateVersionCount` and does not
    /// update the `lastStateChangeAt` timestamp.
    #[wasm_bindgen(js_name = updatableMetadata, getter)]
    pub fn updatable_metadata(&self) -> Option<String> {
        self.0.updatable_metadata.clone()
    }

    /// The timestamp of the most recent state change, in milliseconds since
    /// the Unix epoch.
    #[wasm_bindgen(js_name = lastStateChangeAt, getter)]
    pub fn last_state_change_at(&self) -> u64 {
        self.0.last_state_change_at
    }

    /// The number of state versions the notarization has gone through.
    /// `0` means the state has not been updated since creation.
    #[wasm_bindgen(js_name = stateVersionCount, getter)]
    pub fn state_version_count(&self) -> u64 {
        self.0.state_version_count
    }

    /// The {@link NotarizationMethod} the notarization was created with.
    #[wasm_bindgen(getter)]
    pub fn method(&self) -> WasmNotarizationMethod {
        self.0.method.clone().into()
    }

    /// The current owner's IOTA address.
    #[wasm_bindgen(getter)]
    pub fn owner(&self) -> WasmIotaAddress {
        WasmIotaAddress::from_str(&self.0.owner.to_string())
            .expect("Invalid address stored on-chain, this should never happen")
    }

    /// Creates an IOTA Resource Locator (IRL) builder rooted at this
    /// notarization.
    ///
    /// @remarks
    /// The returned builder produces IRLs of the form
    /// `iota:<network alias or genesis digest>/<notarization ID>/state/data`
    /// and similar paths for related fields.
    ///
    /// @param network - The IOTA network identifier (e.g. `"mainnet"`).
    ///
    /// @returns A {@link NotarizationResourceBuilder} for this notarization.
    ///
    /// @throws When `network` is not a valid IOTA network identifier.
    #[wasm_bindgen(js_name = iotaResourceLocatorBuilder)]
    pub fn iota_resource_locator_builder(
        &self,
        network: &str,
    ) -> std::result::Result<WasmNotarizationResourceBuilder, JsError> {
        let network_name = NetworkName::from_str(network)?;
        Ok(WasmNotarizationResourceBuilder(
            self.0.iota_resource_locator_builder(&network_name),
        ))
    }
}

impl From<OnChainNotarization> for WasmOnChainNotarization {
    fn from(notarization: OnChainNotarization) -> Self {
        WasmOnChainNotarization::new(notarization)
    }
}

/// Builder for IOTA Resource Locators (IRLs) pointing at fields of an
/// {@link OnChainNotarization}.
#[wasm_bindgen(js_name = NotarizationResourceBuilder)]
pub struct WasmNotarizationResourceBuilder(NotarizationResourceBuilder);

#[wasm_bindgen(js_class = NotarizationResourceBuilder)]
impl WasmNotarizationResourceBuilder {
    /// An IRL pointing at the notarization's current state payload.
    pub fn data(&self) -> String {
        self.0.data().to_string()
    }

    /// An IRL pointing at the notarization's immutable metadata.
    #[wasm_bindgen(js_name = immutableMetadata)]
    pub fn immutable_metadata(&self) -> String {
        self.0.immutable_metadata().to_string()
    }

    /// An IRL pointing at the notarization's current state metadata.
    #[wasm_bindgen(js_name = stateMetadata)]
    pub fn state_metadata(&self) -> String {
        self.0.state_metadata().to_string()
    }

    /// An IRL pointing at the notarization's updatable metadata.
    #[wasm_bindgen(js_name = updatableMetadata)]
    pub fn updatable_metadata(&self) -> String {
        self.0.updatable_metadata().to_string()
    }

    /// An IRL pointing at the notarization's owner.
    pub fn owner(&self) -> String {
        self.0.owner().to_string()
    }
}

/// Transaction that creates a Locked-Notarization.
///
/// @remarks
/// A Locked-Notarization is immutable after creation: its state and
/// updatable metadata are fixed for the lifetime of the object. On
/// success the new notarization object is transferred to the transaction
/// sender.
///
/// Emits a `LockedNotarizationCreated` event on success.
#[wasm_bindgen(js_name = CreateNotarizationLocked, inspectable)]
pub struct WasmCreateNotarizationLocked(pub(crate) CreateNotarization<Locked>);

#[wasm_bindgen(js_class = CreateNotarizationLocked)]
impl WasmCreateNotarizationLocked {
    /// Constructs the transaction from a configured Locked-Notarization
    /// builder.
    ///
    /// @param builder - A finalized {@link NotarizationBuilderLocked}.
    #[wasm_bindgen(constructor)]
    pub fn new(builder: WasmNotarizationBuilderLocked) -> Self {
        WasmCreateNotarizationLocked(CreateNotarization::<Locked>::new(builder.0))
    }

    /// Builds the programmable transaction bytes.
    ///
    /// @param client - A read-only client connected to the target network.
    ///
    /// @returns The BCS-serialized programmable transaction, ready to be
    /// signed and submitted.
    ///
    /// @throws When the transaction cannot be built — e.g. when the
    /// configured state, metadata, or lock combination is rejected.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Reads the on-chain effects and events of the submitted transaction
    /// and returns the resulting {@link OnChainNotarization}.
    ///
    /// @remarks
    /// Invoked automatically by the {@link TransactionBuilder} machinery
    /// after the transaction has been submitted; calling it directly is
    /// normally not necessary.
    ///
    /// @param effects - The transaction block effects produced on-chain.
    /// @param events - The transaction block events produced on-chain.
    /// @param client - A read-only client connected to the target network.
    ///
    /// @returns The created {@link OnChainNotarization}.
    ///
    /// @throws When the effects/events are inconsistent with this transaction
    /// or the result cannot be reconstructed.
    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmOnChainNotarization> {
        apply_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}

/// Transaction that creates a Dynamic-Notarization.
///
/// @remarks
/// A Dynamic-Notarization can be updated after creation. On success the new
/// notarization object is transferred to the transaction sender.
///
/// Emits a `DynamicNotarizationCreated` event on success.
#[wasm_bindgen(js_name = CreateNotarizationDynamic, inspectable)]
pub struct WasmCreateNotarizationDynamic(pub(crate) CreateNotarization<Dynamic>);

#[wasm_bindgen(js_class = CreateNotarizationDynamic)]
impl WasmCreateNotarizationDynamic {
    /// Constructs the transaction from a configured Dynamic-Notarization
    /// builder.
    ///
    /// @param builder - A finalized {@link NotarizationBuilderDynamic}.
    #[wasm_bindgen(constructor)]
    pub fn new(builder: WasmNotarizationBuilderDynamic) -> Self {
        WasmCreateNotarizationDynamic(CreateNotarization::<Dynamic>::new(builder.0))
    }

    /// Builds the programmable transaction bytes.
    ///
    /// @param client - A read-only client connected to the target network.
    ///
    /// @returns The BCS-serialized programmable transaction, ready to be
    /// signed and submitted.
    ///
    /// @throws When the transaction cannot be built — e.g. when the
    /// configured state, metadata, or lock combination is rejected.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Reads the on-chain effects and events of the submitted transaction
    /// and returns the resulting {@link OnChainNotarization}.
    ///
    /// @remarks
    /// Invoked automatically by the {@link TransactionBuilder} machinery
    /// after the transaction has been submitted; calling it directly is
    /// normally not necessary.
    ///
    /// @param effects - The transaction block effects produced on-chain.
    /// @param events - The transaction block events produced on-chain.
    /// @param client - A read-only client connected to the target network.
    ///
    /// @returns The created {@link OnChainNotarization}.
    ///
    /// @throws When the effects/events are inconsistent with this transaction
    /// or the result cannot be reconstructed.
    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmOnChainNotarization> {
        apply_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}

/// Transaction that replaces the state of a notarization.
///
/// @remarks
/// On success the transaction increments `stateVersionCount` by one and
/// refreshes `lastStateChangeAt` to the on-chain clock timestamp.
///
/// Behavior depends on the Notarization Method:
/// * `Dynamic`: always permitted — the underlying `updateLock` is fixed to
///   {@link TimeLockType.None}.
/// * `Locked`: always aborts on-chain, because the underlying `updateLock`
///   is pinned to {@link TimeLockType.UntilDestroyed}.
///
/// Emits a `NotarizationUpdated` event on success.
#[wasm_bindgen(js_name = UpdateState, inspectable)]
pub struct WasmUpdateState(pub(crate) UpdateState);

#[wasm_bindgen(js_class = UpdateState)]
impl WasmUpdateState {
    /// Constructs the transaction.
    ///
    /// @param state - The replacement {@link State}.
    /// @param objectId - The notarization object's ID.
    ///
    /// @throws When the ID is malformed.
    #[wasm_bindgen(constructor)]
    pub fn new(state: WasmState, object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        Ok(WasmUpdateState(UpdateState::new(state.0, obj_id)))
    }

    /// Builds the programmable transaction bytes.
    ///
    /// @param client - A read-only client connected to the target network.
    ///
    /// @returns The BCS-serialized programmable transaction, ready to be
    /// signed and submitted.
    ///
    /// @throws When the transaction cannot be built.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Reads the on-chain effects and events of the submitted transaction.
    ///
    /// @remarks
    /// Invoked automatically by the {@link TransactionBuilder} machinery
    /// after the transaction has been submitted; calling it directly is
    /// normally not necessary.
    ///
    /// @param effects - The transaction block effects produced on-chain.
    /// @param events - The transaction block events produced on-chain.
    /// @param client - A read-only client connected to the target network.
    ///
    /// @throws When the effects/events are inconsistent with this transaction.
    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmEmpty> {
        apply_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}

/// Transaction that replaces the updatable metadata of a notarization.
///
/// @remarks
/// Does not affect the state, the `stateVersionCount`, the
/// `lastStateChangeAt` timestamp, or the immutable description.
///
/// Behavior depends on the Notarization Method:
/// * `Dynamic`: always permitted — the underlying `updateLock` is fixed to
///   {@link TimeLockType.None}.
/// * `Locked`: always aborts on-chain, because the underlying `updateLock`
///   is pinned to {@link TimeLockType.UntilDestroyed}.
#[wasm_bindgen(js_name = UpdateMetadata, inspectable)]
pub struct WasmUpdateMetadata(pub(crate) UpdateMetadata);

#[wasm_bindgen(js_class = UpdateMetadata)]
impl WasmUpdateMetadata {
    /// Constructs the transaction.
    ///
    /// @param metadata - The replacement metadata, or `null` to clear it.
    /// @param objectId - The notarization object's ID.
    ///
    /// @throws When the ID is malformed.
    #[wasm_bindgen(constructor)]
    pub fn new(metadata: Option<String>, object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        Ok(WasmUpdateMetadata(UpdateMetadata::new(metadata, obj_id)))
    }

    /// Builds the programmable transaction bytes.
    ///
    /// @param client - A read-only client connected to the target network.
    ///
    /// @returns The BCS-serialized programmable transaction, ready to be
    /// signed and submitted.
    ///
    /// @throws When the transaction cannot be built.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Reads the on-chain effects and events of the submitted transaction.
    ///
    /// @remarks
    /// Invoked automatically by the {@link TransactionBuilder} machinery
    /// after the transaction has been submitted; calling it directly is
    /// normally not necessary.
    ///
    /// @param effects - The transaction block effects produced on-chain.
    /// @param events - The transaction block events produced on-chain.
    /// @param client - A read-only client connected to the target network.
    ///
    /// @throws When the effects/events are inconsistent with this transaction.
    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmEmpty> {
        apply_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}

/// Transaction that destroys a notarization and releases its object ID.
///
/// @remarks
/// The notarization must currently be destroy-allowed (see
/// {@link NotarizationClientReadOnly.isDestroyAllowed}); otherwise the
/// on-chain transaction aborts. All component {@link TimeLock}s of the
/// attached {@link LockMetadata} are destroyed in the process.
/// A {@link TimeLockType.Infinite} lock is not
/// destructible and therefore always blocks destruction.
///
/// Emits a `NotarizationDestroyed` event on success.
#[wasm_bindgen(js_name = DestroyNotarization, inspectable)]
pub struct WasmDestroyNotarization(pub(crate) DestroyNotarization);

#[wasm_bindgen(js_class = DestroyNotarization)]
impl WasmDestroyNotarization {
    /// Constructs the transaction.
    ///
    /// @param objectId - The notarization object's ID.
    ///
    /// @throws When the ID is malformed.
    #[wasm_bindgen(constructor)]
    pub fn new(object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        Ok(WasmDestroyNotarization(DestroyNotarization::new(obj_id)))
    }

    /// Builds the programmable transaction bytes.
    ///
    /// @param client - A read-only client connected to the target network.
    ///
    /// @returns The BCS-serialized programmable transaction, ready to be
    /// signed and submitted.
    ///
    /// @throws When the transaction cannot be built.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Reads the on-chain effects and events of the submitted transaction.
    ///
    /// @remarks
    /// Invoked automatically by the {@link TransactionBuilder} machinery
    /// after the transaction has been submitted; calling it directly is
    /// normally not necessary.
    ///
    /// @param effects - The transaction block effects produced on-chain.
    /// @param events - The transaction block events produced on-chain.
    /// @param client - A read-only client connected to the target network.
    ///
    /// @throws When the effects/events are inconsistent with this transaction.
    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmEmpty> {
        apply_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}

/// Transaction that transfers ownership of a Dynamic-Notarization.
///
/// @remarks
/// Permitted only when the notarization has no {@link LockMetadata} or when
/// its `transferLock` is not currently active. Submitting against a
/// Locked-Notarization or while the transfer lock is engaged aborts on-chain.
///
/// Emits a `DynamicNotarizationTransferred` event on success.
#[wasm_bindgen(js_name = TransferNotarization, inspectable)]
pub struct WasmTransferNotarization(pub(crate) TransferNotarization);

#[wasm_bindgen(js_class = TransferNotarization)]
impl WasmTransferNotarization {
    /// Constructs the transaction.
    ///
    /// @param recipient - The new owner's IOTA address.
    /// @param objectId - The notarization object's ID.
    ///
    /// @throws When the ID or address is malformed.
    #[wasm_bindgen(constructor)]
    pub fn new(recipient: WasmIotaAddress, object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        let recipient_address = parse_wasm_iota_address(&recipient)?;
        Ok(WasmTransferNotarization(TransferNotarization::new(
            recipient_address,
            obj_id,
        )))
    }

    /// Builds the programmable transaction bytes.
    ///
    /// @param client - A read-only client connected to the target network.
    ///
    /// @returns The BCS-serialized programmable transaction, ready to be
    /// signed and submitted.
    ///
    /// @throws When the transaction cannot be built.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Reads the on-chain effects and events of the submitted transaction.
    ///
    /// @remarks
    /// Invoked automatically by the {@link TransactionBuilder} machinery
    /// after the transaction has been submitted; calling it directly is
    /// normally not necessary.
    ///
    /// @param effects - The transaction block effects produced on-chain.
    /// @param events - The transaction block events produced on-chain.
    /// @param client - A read-only client connected to the target network.
    ///
    /// @throws When the effects/events are inconsistent with this transaction.
    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmEmpty> {
        apply_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}
