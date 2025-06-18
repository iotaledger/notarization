// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::prelude::*;

use notarization::core::builder::Locked;
use notarization::core::builder::Dynamic;
use notarization::core::notarization::{CreateNotarization, OnChainNotarization};
use notarization::core::destroy::DestroyNotarization;
use notarization::core::metadata::UpdateMetadata;
use notarization::core::state::UpdateState;
use notarization::core::transfer::TransferNotarization;

use iota_interaction_ts::core_client::WasmCoreClientReadOnly;
use iota_interaction_ts::bindings::WasmIotaTransactionBlockEffects;
use iota_interaction_ts::bindings::WasmIotaTransactionBlockEvents;
use iota_interaction_ts::error::Result;
use product_common::bindings::WasmObjectID;
use product_common::bindings::WasmIotaAddress;
use product_common::bindings::utils::{apply_with_events, build_programmable_transaction,
                                      parse_wasm_iota_address, parse_wasm_object_id};

use crate::wasm_notarization_builder::WasmNotarizationBuilderLocked;
use crate::wasm_notarization_builder::WasmNotarizationBuilderDynamic;
use crate::wasm_types::{WasmState, WasmNotarizationMethod, WasmEmpty};
use crate::wasm_types::WasmImmutableMetadata;

#[wasm_bindgen(js_name = OnChainNotarization, inspectable)]
#[derive(Clone)]
pub struct WasmOnChainNotarization(pub(crate) OnChainNotarization);

#[wasm_bindgen(js_class = OnChainNotarization)]
impl WasmOnChainNotarization {
    pub(crate) fn new(identity: OnChainNotarization) -> Self {Self(identity)}

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String { self.0.id.id.bytes.to_hex() }
    #[wasm_bindgen(getter)]
    pub fn state(&self) -> WasmState { WasmState(self.0.state.clone()) }
    #[wasm_bindgen(js_name = immutableMetadata, getter)]
    pub fn immutable_metadata(&self) -> WasmImmutableMetadata { WasmImmutableMetadata(self.0.immutable_metadata.clone()) }
    #[wasm_bindgen(js_name = updatableMetadata, getter)]
    pub fn updatable_metadata(&self) -> Option<String> {self.0.updatable_metadata.clone()}
    #[wasm_bindgen(js_name = lastStateChangeAt, getter)]
    pub fn last_state_change_at(&self) -> u64 {self.0.last_state_change_at}
    #[wasm_bindgen(js_name = stateVersionCount, getter)]
    pub fn state_version_count(&self) -> u64 {self.0.state_version_count}
    #[wasm_bindgen(getter)]
    pub fn method(&self) -> WasmNotarizationMethod {self.0.method.clone().into()}
}

impl From<OnChainNotarization> for WasmOnChainNotarization {
    fn from(identity: OnChainNotarization) -> Self {
        WasmOnChainNotarization::new(identity)
    }
}

#[wasm_bindgen(js_name = CreateNotarizationLocked, inspectable)]
pub struct WasmCreateNotarizationLocked(pub(crate) CreateNotarization<Locked>);

#[wasm_bindgen(js_class = CreateNotarizationLocked)]
impl WasmCreateNotarizationLocked {
    #[wasm_bindgen(constructor)]
    pub fn new(builder: WasmNotarizationBuilderLocked) -> Self {
        WasmCreateNotarizationLocked(CreateNotarization::<Locked>::new(builder.0))
    }

    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen]
    pub async fn apply(
        self,
        _wasm_effects: &WasmIotaTransactionBlockEffects,
        _client: &WasmCoreClientReadOnly,
    ) -> Result<WasmOnChainNotarization> {
        unimplemented!("Function CreateNotarizationLocked::apply() should never be called.");
    }

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

#[wasm_bindgen(js_name = CreateNotarizationDynamic, inspectable)]
pub struct WasmCreateNotarizationDynamic(pub(crate) CreateNotarization<Dynamic>);

#[wasm_bindgen(js_class = CreateNotarizationDynamic)]
impl WasmCreateNotarizationDynamic {
    #[wasm_bindgen(constructor)]
    pub fn new(builder: WasmNotarizationBuilderDynamic) -> Self {
        WasmCreateNotarizationDynamic(CreateNotarization::<Dynamic>::new(builder.0))
    }

    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen]
    pub async fn apply(
        self,
        _wasm_effects: &WasmIotaTransactionBlockEffects,
        _client: &WasmCoreClientReadOnly,
    ) -> Result<WasmOnChainNotarization> {
        unimplemented!("Function CreateNotarizationDynamic::apply() should never be called.");
    }

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

#[wasm_bindgen(js_name = UpdateState, inspectable)]
pub struct WasmUpdateState(pub(crate) UpdateState);

#[wasm_bindgen(js_class = UpdateState)]
impl WasmUpdateState {
    #[wasm_bindgen(constructor)]
    pub fn new(state: WasmState, object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        Ok(WasmUpdateState(UpdateState::new(state.0, obj_id)))
    }

    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen]
    pub async fn apply(
        self,
        _wasm_effects: &WasmIotaTransactionBlockEffects,
        _client: &WasmCoreClientReadOnly,
    ) -> Result<WasmOnChainNotarization> {
        unimplemented!("Function UpdateState::apply() should never be called.");
    }

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

#[wasm_bindgen(js_name = UpdateMetadata, inspectable)]
pub struct WasmUpdateMetadata(pub(crate) UpdateMetadata);

#[wasm_bindgen(js_class = UpdateMetadata)]
impl WasmUpdateMetadata {
    #[wasm_bindgen(constructor)]
    pub fn new(metadata: Option<String>, object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        Ok(WasmUpdateMetadata(UpdateMetadata::new(metadata, obj_id)))
    }

    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen]
    pub async fn apply(
        self,
        _wasm_effects: &WasmIotaTransactionBlockEffects,
        _client: &WasmCoreClientReadOnly,
    ) -> Result<WasmEmpty> {
        unimplemented!("Function UpdateMetadata::apply() should never be called.");
    }

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

#[wasm_bindgen(js_name = DestroyNotarization, inspectable)]
pub struct WasmDestroyNotarization(pub(crate) DestroyNotarization);

#[wasm_bindgen(js_class = DestroyNotarization)]
impl WasmDestroyNotarization {
    #[wasm_bindgen(constructor)]
    pub fn new(object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        Ok(WasmDestroyNotarization(DestroyNotarization::new(obj_id)))
    }

    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen]
    pub async fn apply(
        self,
        _wasm_effects: &WasmIotaTransactionBlockEffects,
        _client: &WasmCoreClientReadOnly,
    ) -> Result<WasmEmpty> {
        unimplemented!("Function DestroyNotarization::apply() should never be called.");
    }

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

#[wasm_bindgen(js_name = TransferNotarization, inspectable)]
pub struct WasmTransferNotarization(pub(crate) TransferNotarization);

#[wasm_bindgen(js_class = TransferNotarization)]
impl WasmTransferNotarization {
    #[wasm_bindgen(constructor)]
    pub fn new(recipient: WasmIotaAddress, object_id: WasmObjectID) -> Result<Self> {
        let obj_id = parse_wasm_object_id(&object_id)?;
        let recipient_address = parse_wasm_iota_address(&recipient)?;
        Ok(WasmTransferNotarization(TransferNotarization::new(recipient_address, obj_id)))
    }

    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen]
    pub async fn apply(
        self,
        _wasm_effects: &WasmIotaTransactionBlockEffects,
        _client: &WasmCoreClientReadOnly,
    ) -> Result<WasmEmpty> {
        unimplemented!("Function TransferNotarization::apply() should never be called.");
    }

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