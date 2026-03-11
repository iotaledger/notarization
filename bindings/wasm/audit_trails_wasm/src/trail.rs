// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trails::core::create::{CreateTrail, TrailCreated};
use audit_trails::core::records::{AddRecord, DeleteRecord, DeleteRecordsBatch};
use audit_trails::core::trail::UpdateMetadata;
use audit_trails::core::types::{OnChainAuditTrail, RecordAdded, RecordDeleted};
use iota_interaction_ts::bindings::{WasmIotaTransactionBlockEffects, WasmIotaTransactionBlockEvents};
use iota_interaction_ts::core_client::WasmCoreClientReadOnly;
use iota_interaction_ts::wasm_error::{Result, WasmResult};
use js_sys::Object;
use product_common::bindings::core_client::WasmManagedCoreClientReadOnly;
use product_common::transaction::transaction_builder::Transaction;
use wasm_bindgen::prelude::*;

use crate::builder::WasmAuditTrailBuilder;
use crate::types::{WasmEmpty, WasmImmutableMetadata, WasmLockingConfig};
use crate::audit_trails_wasm_result;

#[wasm_bindgen(js_name = OnChainAuditTrail, inspectable)]
#[derive(Clone)]
pub struct WasmOnChainAuditTrail(pub(crate) OnChainAuditTrail);

#[wasm_bindgen(js_class = OnChainAuditTrail)]
impl WasmOnChainAuditTrail {
    pub(crate) fn new(trail: OnChainAuditTrail) -> Self {
        Self(trail)
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.id.to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn creator(&self) -> String {
        self.0.creator.to_string()
    }

    #[wasm_bindgen(js_name = createdAt, getter)]
    pub fn created_at(&self) -> u64 {
        self.0.created_at
    }

    #[wasm_bindgen(js_name = sequenceNumber, getter)]
    pub fn sequence_number(&self) -> u64 {
        self.0.sequence_number
    }

    #[wasm_bindgen(js_name = lockingConfig, getter)]
    pub fn locking_config(&self) -> WasmLockingConfig {
        self.0.locking_config.clone().into()
    }

    #[wasm_bindgen(js_name = immutableMetadata, getter)]
    pub fn immutable_metadata(&self) -> Option<WasmImmutableMetadata> {
        self.0.immutable_metadata.clone().map(Into::into)
    }

    #[wasm_bindgen(js_name = updatableMetadata, getter)]
    pub fn updatable_metadata(&self) -> Option<String> {
        self.0.updatable_metadata.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u64 {
        self.0.version
    }
}

impl From<OnChainAuditTrail> for WasmOnChainAuditTrail {
    fn from(value: OnChainAuditTrail) -> Self {
        Self::new(value)
    }
}

async fn apply_trail_created(
    tx: CreateTrail,
    wasm_effects: &WasmIotaTransactionBlockEffects,
    wasm_events: &WasmIotaTransactionBlockEvents,
    client: &WasmCoreClientReadOnly,
) -> Result<WasmOnChainAuditTrail> {
    let managed_client = WasmManagedCoreClientReadOnly::from_wasm(client)?;
    let mut effects = wasm_effects.clone().into();
    let mut events = wasm_events.clone().into();
    let created = tx.apply_with_events(&mut effects, &mut events, &managed_client).await;

    let rem_wasm_effects = WasmIotaTransactionBlockEffects::from(&effects);
    Object::assign(wasm_effects, &rem_wasm_effects);
    let rem_wasm_events = WasmIotaTransactionBlockEvents::from(&events);
    Object::assign(wasm_events, &rem_wasm_events);

    let created: TrailCreated = audit_trails_wasm_result(created)?;
    let trail = audit_trails_wasm_result(created.fetch_audit_trail(&managed_client).await)?;
    Ok(trail.into())
}

async fn build_audit_trail_transaction<T>(tx: &T, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>>
where
    T: Transaction<Error = audit_trails::error::Error>,
{
    let managed_client = WasmManagedCoreClientReadOnly::from_wasm(client)?;
    let pt = audit_trails_wasm_result(tx.build_programmable_transaction(&managed_client).await)?;
    bcs::to_bytes(&pt).wasm_result()
}

async fn apply_audit_trail_with_events<T, O>(
    tx: T,
    wasm_effects: &WasmIotaTransactionBlockEffects,
    wasm_events: &WasmIotaTransactionBlockEvents,
    client: &WasmCoreClientReadOnly,
) -> Result<O>
where
    T: Transaction<Error = audit_trails::error::Error>,
    O: From<<T as Transaction>::Output>,
{
    let managed_client = WasmManagedCoreClientReadOnly::from_wasm(client)?;
    let mut effects = wasm_effects.clone().into();
    let mut events = wasm_events.clone().into();
    let output = tx.apply_with_events(&mut effects, &mut events, &managed_client).await;

    let rem_wasm_effects = WasmIotaTransactionBlockEffects::from(&effects);
    Object::assign(wasm_effects, &rem_wasm_effects);
    let rem_wasm_events = WasmIotaTransactionBlockEvents::from(&events);
    Object::assign(wasm_events, &rem_wasm_events);

    audit_trails_wasm_result(output).map(Into::into)
}

#[wasm_bindgen(js_name = CreateTrail, inspectable)]
pub struct WasmCreateTrail(pub(crate) CreateTrail);

#[wasm_bindgen(js_class = CreateTrail)]
impl WasmCreateTrail {
    #[wasm_bindgen(constructor)]
    pub fn new(builder: WasmAuditTrailBuilder) -> Self {
        Self(CreateTrail::new(builder.0))
    }

    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_audit_trail_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmOnChainAuditTrail> {
        apply_trail_created(self.0, wasm_effects, wasm_events, client).await
    }
}

#[wasm_bindgen(js_name = UpdateMetadata, inspectable)]
pub struct WasmUpdateMetadata(pub(crate) UpdateMetadata);

#[wasm_bindgen(js_class = UpdateMetadata)]
impl WasmUpdateMetadata {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_audit_trail_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmEmpty> {
        apply_audit_trail_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}

#[wasm_bindgen(js_name = AddRecord, inspectable)]
pub struct WasmAddRecord(pub(crate) AddRecord);

#[wasm_bindgen(js_class = AddRecord)]
impl WasmAddRecord {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_audit_trail_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<u64> {
        let added: RecordAdded =
            apply_audit_trail_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(added.sequence_number)
    }
}

#[wasm_bindgen(js_name = DeleteRecord, inspectable)]
pub struct WasmDeleteRecord(pub(crate) DeleteRecord);

#[wasm_bindgen(js_class = DeleteRecord)]
impl WasmDeleteRecord {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_audit_trail_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<u64> {
        let deleted: RecordDeleted =
            apply_audit_trail_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(deleted.sequence_number)
    }
}

#[wasm_bindgen(js_name = DeleteRecordsBatch, inspectable)]
pub struct WasmDeleteRecordsBatch(pub(crate) DeleteRecordsBatch);

#[wasm_bindgen(js_class = DeleteRecordsBatch)]
impl WasmDeleteRecordsBatch {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_audit_trail_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<u64> {
        apply_audit_trail_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}
