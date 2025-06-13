// Copyright 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::rpc_types::IotaTransactionBlockEvents;
use js_sys::Object;
use wasm_bindgen::prelude::*;

use notarization::core::builder::Locked;
use notarization::core::builder::Dynamic;
use notarization::core::notarization::{CreateNotarization, OnChainNotarization};

use iota_interaction_ts::core_client::WasmCoreClientReadOnly;
use iota_interaction_ts::bindings::WasmIotaTransactionBlockEffects;
use iota_interaction_ts::bindings::WasmIotaTransactionBlockEvents;
use iota_interaction_ts::error::{Result, WasmResult};
use product_common::transaction::transaction_builder::Transaction;
use product_common::bindings::core_client::WasmManagedCoreClientReadOnly;
use crate::wasm_notarization_builder::WasmNotarizationBuilderLocked;
use crate::wasm_notarization_builder::WasmNotarizationBuilderDynamic;
use crate::wasm_types::{WasmState};
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
    #[wasm_bindgen(js_name = updateableMetadata, getter)]
    pub fn updateable_metadata(&self) -> Option<String> {self.0.updateable_metadata.clone()}
    #[wasm_bindgen(js_name = lastStateChangeAt, getter)]
    pub fn last_state_change_at(&self) -> u64 {self.0.last_state_change_at}
    #[wasm_bindgen(js_name = stateVersionCount, getter)]
    pub fn state_version_count(&self) -> u64 {self.0.state_version_count}
    #[wasm_bindgen(getter)]
    pub fn method(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.0.method).unwrap()
    }
}

async fn apply_with_events<M: Clone>(
    notarization: CreateNotarization<M>,
    wasm_effects: &WasmIotaTransactionBlockEffects,
    wasm_events: &WasmIotaTransactionBlockEvents,
    client: &WasmCoreClientReadOnly,
) -> Result<WasmOnChainNotarization> {
    let managed_client = WasmManagedCoreClientReadOnly::from_wasm(client)?;
    let mut effects = wasm_effects.clone().into();
    let mut events = wasm_events.clone().into();
    let apply_result = notarization.apply_with_events(&mut effects, &mut events, &managed_client).await;
    let rem_wasm_effects = WasmIotaTransactionBlockEffects::from(&effects);
    Object::assign(wasm_effects, &rem_wasm_effects);
    let rem_wasm_events = WasmIotaTransactionBlockEvents::from(&events);
    Object::assign(wasm_events, &rem_wasm_events);
    apply_result.wasm_result().map(WasmOnChainNotarization::new)
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
        let managed_client = WasmManagedCoreClientReadOnly::from_wasm(client)?;
        let pt = self
            .0
            .build_programmable_transaction(&managed_client)
            .await
            .wasm_result()?;
        bcs::to_bytes(&pt).wasm_result()
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
        let managed_client = WasmManagedCoreClientReadOnly::from_wasm(client)?;
        let pt = self
            .0
            .build_programmable_transaction(&managed_client)
            .await
            .wasm_result()?;
        bcs::to_bytes(&pt).wasm_result()
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

