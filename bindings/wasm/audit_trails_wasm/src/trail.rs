// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trails::core::access::{
    CleanupRevokedCapabilities, CreateRole, DeleteRole, DestroyCapability, DestroyInitialAdminCapability,
    IssueCapability, RevokeCapability, RevokeInitialAdminCapability, UpdateRole,
};
use audit_trails::core::create::{CreateTrail, TrailCreated};
use audit_trails::core::locking::{
    UpdateDeleteRecordWindow, UpdateDeleteTrailLock, UpdateLockingConfig, UpdateWriteLock,
};
use audit_trails::core::records::{AddRecord, DeleteRecord, DeleteRecordsBatch};
use audit_trails::core::tags::{AddRecordTag, RemoveRecordTag};
use audit_trails::core::trail::{DeleteAuditTrail, Migrate, UpdateMetadata};
use audit_trails::core::types::{
    AuditTrailDeleted, CapabilityDestroyed, CapabilityIssued, CapabilityRevoked, OnChainAuditTrail, RecordAdded,
    RecordDeleted, RoleCreated, RoleDeleted, RoleUpdated,
};
use iota_interaction_ts::bindings::{WasmIotaTransactionBlockEffects, WasmIotaTransactionBlockEvents};
use iota_interaction_ts::core_client::WasmCoreClientReadOnly;
use iota_interaction_ts::wasm_error::{Result, WasmResult};
use product_common::bindings::core_client::WasmManagedCoreClientReadOnly;
use product_common::bindings::utils::{apply_with_events, build_programmable_transaction};
use wasm_bindgen::prelude::*;

use crate::builder::WasmAuditTrailBuilder;
use crate::types::{
    WasmAuditTrailDeleted, WasmCapabilityDestroyed, WasmCapabilityIssued, WasmCapabilityRevoked, WasmEmpty,
    WasmImmutableMetadata, WasmLinkedTable, WasmLockingConfig, WasmRecordAdded, WasmRecordDeleted, WasmRecordTagEntry,
    WasmRoleCreated, WasmRoleDeleted, WasmRoleMap, WasmRoleUpdated,
};

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

    #[wasm_bindgen(getter)]
    pub fn records(&self) -> WasmLinkedTable {
        self.0.records.clone().into()
    }

    #[wasm_bindgen(getter)]
    pub fn tags(&self) -> Vec<WasmRecordTagEntry> {
        let mut tags: Vec<WasmRecordTagEntry> = self
            .0
            .tags
            .iter()
            .map(|(tag, usage_count)| (tag.clone(), *usage_count).into())
            .collect();
        tags.sort_unstable_by(|left, right| left.tag.cmp(&right.tag));
        tags
    }

    #[wasm_bindgen(getter)]
    pub fn roles(&self) -> WasmRoleMap {
        self.0.roles.clone().into()
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
    let created: TrailCreated = apply_with_events(tx, wasm_effects, wasm_events, client).await?;
    let trail = created.fetch_audit_trail(&managed_client).await.wasm_result()?;
    Ok(trail.into())
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
        build_programmable_transaction(&self.0, client).await
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
        build_programmable_transaction(&self.0, client).await
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

#[wasm_bindgen(js_name = Migrate, inspectable)]
pub struct WasmMigrate(pub(crate) Migrate);

#[wasm_bindgen(js_class = Migrate)]
impl WasmMigrate {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
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

#[wasm_bindgen(js_name = DeleteAuditTrail, inspectable)]
pub struct WasmDeleteAuditTrail(pub(crate) DeleteAuditTrail);

#[wasm_bindgen(js_class = DeleteAuditTrail)]
impl WasmDeleteAuditTrail {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmAuditTrailDeleted> {
        let event: AuditTrailDeleted = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(event.into())
    }
}

#[wasm_bindgen(js_name = UpdateLockingConfig, inspectable)]
pub struct WasmUpdateLockingConfig(pub(crate) UpdateLockingConfig);

#[wasm_bindgen(js_class = UpdateLockingConfig)]
impl WasmUpdateLockingConfig {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
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

#[wasm_bindgen(js_name = UpdateDeleteRecordWindow, inspectable)]
pub struct WasmUpdateDeleteRecordWindow(pub(crate) UpdateDeleteRecordWindow);

#[wasm_bindgen(js_class = UpdateDeleteRecordWindow)]
impl WasmUpdateDeleteRecordWindow {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
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

#[wasm_bindgen(js_name = UpdateDeleteTrailLock, inspectable)]
pub struct WasmUpdateDeleteTrailLock(pub(crate) UpdateDeleteTrailLock);

#[wasm_bindgen(js_class = UpdateDeleteTrailLock)]
impl WasmUpdateDeleteTrailLock {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
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

#[wasm_bindgen(js_name = UpdateWriteLock, inspectable)]
pub struct WasmUpdateWriteLock(pub(crate) UpdateWriteLock);

#[wasm_bindgen(js_class = UpdateWriteLock)]
impl WasmUpdateWriteLock {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
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

#[wasm_bindgen(js_name = CreateRole, inspectable)]
pub struct WasmCreateRole(pub(crate) CreateRole);

#[wasm_bindgen(js_class = CreateRole)]
impl WasmCreateRole {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmRoleCreated> {
        let event: RoleCreated = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(event.into())
    }
}

#[wasm_bindgen(js_name = UpdateRole, inspectable)]
pub struct WasmUpdateRole(pub(crate) UpdateRole);

#[wasm_bindgen(js_class = UpdateRole)]
impl WasmUpdateRole {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmRoleUpdated> {
        let event: RoleUpdated = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(event.into())
    }
}

#[wasm_bindgen(js_name = DeleteRole, inspectable)]
pub struct WasmDeleteRole(pub(crate) DeleteRole);

#[wasm_bindgen(js_class = DeleteRole)]
impl WasmDeleteRole {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmRoleDeleted> {
        let event: RoleDeleted = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(event.into())
    }
}

#[wasm_bindgen(js_name = IssueCapability, inspectable)]
pub struct WasmIssueCapability(pub(crate) IssueCapability);

#[wasm_bindgen(js_class = IssueCapability)]
impl WasmIssueCapability {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmCapabilityIssued> {
        let event: CapabilityIssued = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(event.into())
    }
}

#[wasm_bindgen(js_name = RevokeCapability, inspectable)]
pub struct WasmRevokeCapability(pub(crate) RevokeCapability);

#[wasm_bindgen(js_class = RevokeCapability)]
impl WasmRevokeCapability {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmCapabilityRevoked> {
        let event: CapabilityRevoked = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(event.into())
    }
}

#[wasm_bindgen(js_name = DestroyCapability, inspectable)]
pub struct WasmDestroyCapability(pub(crate) DestroyCapability);

#[wasm_bindgen(js_class = DestroyCapability)]
impl WasmDestroyCapability {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmCapabilityDestroyed> {
        let event: CapabilityDestroyed = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(event.into())
    }
}

#[wasm_bindgen(js_name = DestroyInitialAdminCapability, inspectable)]
pub struct WasmDestroyInitialAdminCapability(pub(crate) DestroyInitialAdminCapability);

#[wasm_bindgen(js_class = DestroyInitialAdminCapability)]
impl WasmDestroyInitialAdminCapability {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmCapabilityDestroyed> {
        let event: CapabilityDestroyed = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(event.into())
    }
}

#[wasm_bindgen(js_name = RevokeInitialAdminCapability, inspectable)]
pub struct WasmRevokeInitialAdminCapability(pub(crate) RevokeInitialAdminCapability);

#[wasm_bindgen(js_class = RevokeInitialAdminCapability)]
impl WasmRevokeInitialAdminCapability {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmCapabilityRevoked> {
        let event: CapabilityRevoked = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(event.into())
    }
}

#[wasm_bindgen(js_name = CleanupRevokedCapabilities, inspectable)]
pub struct WasmCleanupRevokedCapabilities(pub(crate) CleanupRevokedCapabilities);

#[wasm_bindgen(js_class = CleanupRevokedCapabilities)]
impl WasmCleanupRevokedCapabilities {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
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

#[wasm_bindgen(js_name = AddRecord, inspectable)]
pub struct WasmAddRecord(pub(crate) AddRecord);

#[wasm_bindgen(js_class = AddRecord)]
impl WasmAddRecord {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmRecordAdded> {
        let added: RecordAdded = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(added.into())
    }
}

#[wasm_bindgen(js_name = DeleteRecord, inspectable)]
pub struct WasmDeleteRecord(pub(crate) DeleteRecord);

#[wasm_bindgen(js_class = DeleteRecord)]
impl WasmDeleteRecord {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<WasmRecordDeleted> {
        let deleted: RecordDeleted = apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(deleted.into())
    }
}

#[wasm_bindgen(js_name = DeleteRecordsBatch, inspectable)]
pub struct WasmDeleteRecordsBatch(pub(crate) DeleteRecordsBatch);

#[wasm_bindgen(js_class = DeleteRecordsBatch)]
impl WasmDeleteRecordsBatch {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    #[wasm_bindgen(js_name = applyWithEvents)]
    pub async fn apply_with_events(
        self,
        wasm_effects: &WasmIotaTransactionBlockEffects,
        wasm_events: &WasmIotaTransactionBlockEvents,
        client: &WasmCoreClientReadOnly,
    ) -> Result<u64> {
        apply_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}

#[wasm_bindgen(js_name = AddRecordTag, inspectable)]
pub struct WasmAddRecordTag(pub(crate) AddRecordTag);

#[wasm_bindgen(js_class = AddRecordTag)]
impl WasmAddRecordTag {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
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

#[wasm_bindgen(js_name = RemoveRecordTag, inspectable)]
pub struct WasmRemoveRecordTag(pub(crate) RemoveRecordTag);

#[wasm_bindgen(js_class = RemoveRecordTag)]
impl WasmRemoveRecordTag {
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
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
