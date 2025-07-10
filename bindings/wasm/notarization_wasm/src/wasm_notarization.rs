// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction_ts::bindings::{WasmIotaTransactionBlockEffects, WasmIotaTransactionBlockEvents};
use iota_interaction_ts::core_client::WasmCoreClientReadOnly;
use iota_interaction_ts::wasm_error::Result;
use notarization::core::builder::{Dynamic, Locked};
use notarization::core::transactions::{
    CreateNotarization, DestroyNotarization, TransferNotarization, UpdateMetadata, UpdateState,
};
use notarization::core::types::OnChainNotarization;
use product_common::bindings::utils::{
    apply_with_events, build_programmable_transaction, parse_wasm_iota_address, parse_wasm_object_id,
};
use product_common::bindings::{WasmIotaAddress, WasmObjectID};
use wasm_bindgen::prelude::*;

use crate::wasm_notarization_builder::{WasmNotarizationBuilderDynamic, WasmNotarizationBuilderLocked};
use crate::wasm_types::{WasmEmpty, WasmImmutableMetadata, WasmNotarizationMethod, WasmState};

/// Represents an on-chain notarization object.
///
/// Provides access to various properties of the notarization, such as its ID, state, metadata
/// and method.
#[wasm_bindgen(js_name = OnChainNotarization, inspectable)]
#[derive(Clone)]
pub struct WasmOnChainNotarization(pub(crate) OnChainNotarization);

#[wasm_bindgen(js_class = OnChainNotarization)]
impl WasmOnChainNotarization {
    // Creates a new `OnChainNotarization` instance.
    //
    // # Arguments
    // * `notarization` - The `OnChainNotarization` object to wrap.
    pub(crate) fn new(notarization: OnChainNotarization) -> Self {
        Self(notarization)
    }

    /// Retrieves the ID of the notarization.
    ///
    /// # Returns
    /// A hexadecimal string representing the notarization ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.id.bytes.to_hex()
    }

    /// Retrieves the current state of the notarization.
    ///
    /// # Returns
    /// A `State` object representing the notarization state.
    #[wasm_bindgen(getter)]
    pub fn state(&self) -> WasmState {
        WasmState(self.0.state.clone())
    }

    /// Retrieves the immutable metadata of the notarization.
    ///
    /// # Returns
    /// A `ImmutableMetadata` object containing the metadata.
    #[wasm_bindgen(js_name = immutableMetadata, getter)]
    pub fn immutable_metadata(&self) -> WasmImmutableMetadata {
        WasmImmutableMetadata(self.0.immutable_metadata.clone())
    }

    /// Retrieves the updatable metadata of the notarization.
    ///
    /// # Returns
    /// An optional string containing the metadata.
    #[wasm_bindgen(js_name = updatableMetadata, getter)]
    pub fn updatable_metadata(&self) -> Option<String> {
        self.0.updatable_metadata.clone()
    }

    /// Retrieves the timestamp of the last state change.
    ///
    /// # Returns
    /// A `number` value representing the timestamp,
    /// the time in seconds since the Unix epoch.
    #[wasm_bindgen(js_name = lastStateChangeAt, getter)]
    pub fn last_state_change_at(&self) -> u64 {
        self.0.last_state_change_at
    }

    /// Retrieves the count of state versions.
    ///
    /// # Returns
    /// A `number` value representing the number of state versions.
    #[wasm_bindgen(js_name = stateVersionCount, getter)]
    pub fn state_version_count(&self) -> u64 {
        self.0.state_version_count
    }

    /// Retrieves the notarization method.
    ///
    /// # Returns
    /// A `NotarizationMethod` object representing the method.
    #[wasm_bindgen(getter)]
    pub fn method(&self) -> WasmNotarizationMethod {
        self.0.method.clone().into()
    }
}

// Converts an `OnChainNotarization` into a `WasmOnChainNotarization`.
impl From<OnChainNotarization> for WasmOnChainNotarization {
    fn from(notarization: OnChainNotarization) -> Self {
        WasmOnChainNotarization::new(notarization)
    }
}

/// Represents a transaction for creating locked notarization's.
///
/// Locked notarization's cannot be modified after creation, ensuring data permanence.
#[wasm_bindgen(js_name = CreateNotarizationLocked, inspectable)]
pub struct WasmCreateNotarizationLocked(pub(crate) CreateNotarization<Locked>);

#[wasm_bindgen(js_class = CreateNotarizationLocked)]
impl WasmCreateNotarizationLocked {
    #[wasm_bindgen(constructor)]
    pub fn new(builder: WasmNotarizationBuilderLocked) -> Self {
        WasmCreateNotarizationLocked(CreateNotarization::<Locked>::new(builder.0))
    }

    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    /// Builds and returns a programmable transaction for creating a locked notarization.
    ///
    /// # Returns
    /// The binary BCS serialization of the programmable transaction.
    /// This transaction can be submitted to the network to create a new locked notarization.
    ///
    /// # Errors
    /// Returns an error if the transaction cannot be built due to invalid parameters
    /// or other constraints.
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events to this notarization creation operation.
    ///
    /// This method is called automatically by Transaction::build_programmable_transaction()
    /// and Transaction::apply() methods after the transaction has been successfully submitted
    /// to process the results from the ledger.
    ///
    /// # Arguments
    /// * `effects` - The transaction block effects to apply.
    /// * `events` - The transaction block events to apply.
    ///
    /// # Returns
    /// The created notarization ID if successful.
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

/// Represents a transaction for creating dynamic notarization's.
///
/// Dynamic notarization's can be updated after creation, with modification capabilities
/// for both state and metadata.
#[wasm_bindgen(js_name = CreateNotarizationDynamic, inspectable)]
pub struct WasmCreateNotarizationDynamic(pub(crate) CreateNotarization<Dynamic>);

#[wasm_bindgen(js_class = CreateNotarizationDynamic)]
impl WasmCreateNotarizationDynamic {
    #[wasm_bindgen(constructor)]
    pub fn new(builder: WasmNotarizationBuilderDynamic) -> Self {
        WasmCreateNotarizationDynamic(CreateNotarization::<Dynamic>::new(builder.0))
    }

    /// Builds and returns a programmable transaction for creating a dynamic notarization.
    ///
    /// # Returns
    /// The binary BCS serialization of the programmable transaction.
    /// This transaction can be submitted to the network to create a new dynamic notarization.
    ///
    /// # Errors
    /// Returns an error if the transaction cannot be built due to invalid parameters
    /// or other constraints.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events to this notarization creation operation.
    ///
    ///
    /// This method is called automatically by Transaction::build_programmable_transaction()
    /// and Transaction::apply() methods after the transaction has been successfully submitted
    /// to process the results from the ledger.
    ///
    /// # Arguments
    /// * `effects` - The transaction block effects to apply.
    /// * `events` - The transaction block events to apply.
    ///
    /// # Returns
    /// The created notarization ID if successful.
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

/// Represents a transaction for updating the state of a dynamic notarization.
///
/// This is only available for dynamic notarization's
#[wasm_bindgen(js_name = UpdateState, inspectable)]
pub struct WasmUpdateState(pub(crate) UpdateState);

#[wasm_bindgen(js_class = UpdateState)]
impl WasmUpdateState {
    #[wasm_bindgen(constructor)]
    pub fn new(state: WasmState, object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        Ok(WasmUpdateState(UpdateState::new(state.0, obj_id)))
    }

    /// Builds and returns a programmable transaction for updating the state of a notarization.
    ///
    /// # Returns
    /// The binary BCS serialization of the programmable transaction.
    /// This transaction can be submitted to the network to updating the state of a notarization.
    ///
    /// # Errors
    /// Returns an error if the transaction cannot be built due to invalid parameters
    /// or other constraints.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events to this state update operation.
    ///
    /// This method is called automatically by Transaction::build_programmable_transaction()
    /// and Transaction::apply() methods after the transaction has been successfully submitted
    /// to process the results from the ledger.
    ///
    /// # Arguments
    /// * `effects` - The transaction block effects to apply.
    /// * `events` - The transaction block events to apply.
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

/// Represents a transaction for updating the metadata of a notarization.
#[wasm_bindgen(js_name = UpdateMetadata, inspectable)]
pub struct WasmUpdateMetadata(pub(crate) UpdateMetadata);

#[wasm_bindgen(js_class = UpdateMetadata)]
impl WasmUpdateMetadata {
    #[wasm_bindgen(constructor)]
    pub fn new(metadata: Option<String>, object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        Ok(WasmUpdateMetadata(UpdateMetadata::new(metadata, obj_id)))
    }

    /// Builds and returns a programmable transaction for updating the metadata of a notarization.
    ///
    /// # Returns
    /// The binary BCS serialization of the programmable transaction.
    /// This transaction can be submitted to the network to update the metadata of a notarization.
    ///
    /// # Errors
    /// Returns an error if the transaction cannot be built due to invalid parameters
    /// or other constraints.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events to this metadata update operation.
    ///
    /// This method is called automatically by Transaction::build_programmable_transaction()
    /// and Transaction::apply() methods after the transaction has been successfully submitted
    /// to process the results from the ledger.
    ///
    /// # Arguments
    /// * `effects` - The transaction block effects to apply.
    /// * `events` - The transaction block events to apply.
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

/// Represents a transaction for deleting a notarization.
#[wasm_bindgen(js_name = DestroyNotarization, inspectable)]
pub struct WasmDestroyNotarization(pub(crate) DestroyNotarization);

#[wasm_bindgen(js_class = DestroyNotarization)]
impl WasmDestroyNotarization {
    #[wasm_bindgen(constructor)]
    pub fn new(object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        Ok(WasmDestroyNotarization(DestroyNotarization::new(obj_id)))
    }

    /// Builds and returns a programmable transaction for deleting a notarization.
    ///
    /// # Returns
    /// The binary BCS serialization of the programmable transaction.
    /// This transaction can be submitted to the network to delete a notarization.
    ///
    /// # Errors
    /// Returns an error if the transaction cannot be built due to invalid parameters
    /// or other constraints.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events to this notarization delete operation.
    ///
    /// This method is called automatically by Transaction::build_programmable_transaction()
    /// and Transaction::apply() methods after the transaction has been successfully submitted
    /// to process the results from the ledger.
    ///
    /// # Arguments
    /// * `effects` - The transaction block effects to apply.
    /// * `events` - The transaction block events to apply.
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

/// Represents a transaction for transferring a dynamic notarization to a new owner.
///
/// This is only available for dynamic notarization's
#[wasm_bindgen(js_name = TransferNotarization, inspectable)]
pub struct WasmTransferNotarization(pub(crate) TransferNotarization);

#[wasm_bindgen(js_class = TransferNotarization)]
impl WasmTransferNotarization {
    #[wasm_bindgen(constructor)]
    pub fn new(recipient: WasmIotaAddress, object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        let recipient_address = parse_wasm_iota_address(&recipient)?;
        Ok(WasmTransferNotarization(TransferNotarization::new(
            recipient_address,
            obj_id,
        )))
    }

    /// Builds and returns a programmable transaction for transferring a notarization.
    ///
    /// # Returns
    /// The binary BCS serialization of the programmable transaction.
    /// This transaction can be submitted to the network to transfer a notarization.
    ///
    /// # Errors
    /// Returns an error if the transaction cannot be built due to invalid parameters
    /// or other constraints.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events to this transfer operation.
    ///
    /// This method is called automatically by Transaction::build_programmable_transaction()
    /// and Transaction::apply() methods after the transaction has been successfully submitted
    /// to process the results from the ledger.
    ///
    /// # Arguments
    /// * `effects` - The transaction block effects to apply.
    /// * `events` - The transaction block events to apply.
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
