// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use audit_trail::core::access::{
    CleanupRevokedCapabilities, CreateRole, DeleteRole, DestroyCapability, DestroyInitialAdminCapability,
    IssueCapability, RevokeCapability, RevokeInitialAdminCapability, UpdateRole,
};
use audit_trail::core::create::{CreateTrail, TrailCreated};
use audit_trail::core::locking::{
    UpdateDeleteRecordWindow, UpdateDeleteTrailLock, UpdateLockingConfig, UpdateWriteLock,
};
use audit_trail::core::records::{AddRecord, DeleteRecord, DeleteRecordsBatch};
use audit_trail::core::tags::{AddRecordTag, RemoveRecordTag};
use audit_trail::core::trail::{DeleteAuditTrail, Migrate, UpdateMetadata};
use audit_trail::core::types::{
    AuditTrailDeleted, CapabilityDestroyed, CapabilityIssued, CapabilityRevoked, OnChainAuditTrail, RecordAdded,
    RecordDeleted, RevokedCapabilitiesCleanedUp, RoleCreated, RoleDeleted, RoleUpdated,
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
    WasmRevokedCapabilitiesCleanedUp, WasmRoleCreated, WasmRoleDeleted, WasmRoleMap, WasmRoleUpdated,
};

/// Read-only view of an on-chain audit trail for wasm consumers.
#[wasm_bindgen(js_name = OnChainAuditTrail, inspectable)]
#[derive(Clone)]
pub struct WasmOnChainAuditTrail(pub(crate) OnChainAuditTrail);

#[wasm_bindgen(js_class = OnChainAuditTrail)]
impl WasmOnChainAuditTrail {
    pub(crate) fn new(trail: OnChainAuditTrail) -> Self {
        Self(trail)
    }

    /// Returns the trail object ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.id.to_string()
    }

    /// Returns the creator address.
    #[wasm_bindgen(getter)]
    pub fn creator(&self) -> String {
        self.0.creator.to_string()
    }

    /// Returns the creation timestamp in milliseconds.
    #[wasm_bindgen(js_name = createdAt, getter)]
    pub fn created_at(&self) -> u64 {
        self.0.created_at
    }

    /// Returns the current record sequence counter.
    #[wasm_bindgen(js_name = sequenceNumber, getter)]
    pub fn sequence_number(&self) -> u64 {
        self.0.sequence_number
    }

    /// Returns the active locking configuration.
    #[wasm_bindgen(js_name = lockingConfig, getter)]
    pub fn locking_config(&self) -> WasmLockingConfig {
        self.0.locking_config.clone().into()
    }

    /// Returns the record linked-table metadata.
    #[wasm_bindgen(getter)]
    pub fn records(&self) -> WasmLinkedTable {
        self.0.records.clone().into()
    }

    /// Returns the trail-owned record tags together with usage counts.
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

    /// Returns the trail role map.
    #[wasm_bindgen(getter)]
    pub fn roles(&self) -> WasmRoleMap {
        self.0.roles.clone().into()
    }

    /// Returns immutable metadata when present.
    #[wasm_bindgen(js_name = immutableMetadata, getter)]
    pub fn immutable_metadata(&self) -> Option<WasmImmutableMetadata> {
        self.0.immutable_metadata.clone().map(Into::into)
    }

    /// Returns mutable metadata when present.
    #[wasm_bindgen(js_name = updatableMetadata, getter)]
    pub fn updatable_metadata(&self) -> Option<String> {
        self.0.updatable_metadata.clone()
    }

    /// Returns the on-chain version of the trail object.
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

/// Transaction wrapper for trail creation.
///
/// On execution the audit-trail package shares the new trail object, seeds the reserved `Admin` role,
/// transfers a fresh initial-admin capability to the admin address, optionally stores the initial
/// record at sequence number 0 (validating its tag against the registry), and emits an
/// `AuditTrailCreated` event.
#[wasm_bindgen(js_name = CreateTrail, inspectable)]
pub struct WasmCreateTrail(pub(crate) CreateTrail);

#[wasm_bindgen(js_class = CreateTrail)]
impl WasmCreateTrail {
    /// Creates a transaction wrapper from an [`AuditTrailBuilder`](crate::builder::WasmAuditTrailBuilder).
    #[wasm_bindgen(constructor)]
    pub fn new(builder: WasmAuditTrailBuilder) -> Self {
        Self(CreateTrail::new(builder.0))
    }

    /// Builds the programmable transaction bytes for submission.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and then fetches the created trail object.
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

/// Transaction wrapper for mutable-metadata updates.
///
/// Requires the `UpdateMetadata` permission. Passing `null`/`undefined` for the new metadata clears
/// the field on-chain.
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

/// Transaction wrapper for trail migration.
///
/// Requires the `Migrate` permission. Succeeds only when the on-chain trail's package version is
/// strictly less than the package version this binding targets.
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

/// Transaction wrapper for deleting a trail.
///
/// Requires the `DeleteAuditTrail` permission. Aborts on-chain when records still exist or while the
/// configured trail-delete time lock is active. Emits `AuditTrailDeleted` on success.
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

/// Transaction wrapper for replacing the full locking configuration.
///
/// Requires the `UpdateLockingConfig` permission. The supplied configuration's `deleteTrailLock` must
/// not be `TimeLock.withUntilDestroyed()`; the call aborts on-chain otherwise.
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

/// Transaction wrapper for updating the delete-record window.
///
/// Requires the `UpdateLockingConfigForDeleteRecord` permission. Updates only the rule that locks
/// individual records against deletion (time-based or count-based).
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

/// Transaction wrapper for updating the delete-trail lock.
///
/// Requires the `UpdateLockingConfigForDeleteTrail` permission. The new lock must not be
/// `TimeLock.withUntilDestroyed()`.
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

/// Transaction wrapper for updating the write lock.
///
/// Requires the `UpdateLockingConfigForWrite` permission. While the new lock is active, `addRecord`
/// aborts on-chain.
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

/// Transaction wrapper for creating a role.
///
/// Requires the `AddRoles` permission. Any `roleTags` supplied must already exist in the trail's
/// record-tag registry; the on-chain call aborts otherwise. Emits a `RoleCreated` event on success.
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

/// Transaction wrapper for updating a role.
///
/// Requires the `UpdateRoles` permission. Replaces both the role's permissions and its `roleTags`;
/// any newly supplied tag must already be in the trail's record-tag registry. Emits a `RoleUpdated`
/// event on success.
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

/// Transaction wrapper for deleting a role.
///
/// Requires the `DeleteRoles` permission. The reserved initial-admin role (`"Admin"`) cannot be
/// deleted. Emits a `RoleDeleted` event on success.
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

/// Transaction wrapper for issuing a capability.
///
/// Requires the `AddCapabilities` permission. Mints a new capability for the role and transfers it
/// to `issuedTo` (or the caller when absent). `validFromMs` and `validUntilMs` configure usage
/// restrictions enforced when the capability is later presented for authorization. Emits a
/// `CapabilityIssued` event on success.
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

/// Transaction wrapper for revoking a capability.
///
/// Requires the `RevokeCapabilities` permission. Adds the capability ID to the trail's denylist.
/// Pass `capabilityValidUntil` so [`WasmCleanupRevokedCapabilities`] can later prune the entry once
/// that timestamp elapses; pass `null` to keep the denylist entry permanently. Emits a
/// `CapabilityRevoked` event on success.
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

/// Transaction wrapper for destroying a capability.
///
/// Requires the `RevokeCapabilities` permission. Consumes the owned capability object. This path is
/// for ordinary capabilities only — initial-admin capabilities must use
/// [`WasmDestroyInitialAdminCapability`]. Emits a `CapabilityDestroyed` event on success.
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

/// Transaction wrapper for destroying an initial-admin capability.
///
/// Self-service: the holder consumes their own initial-admin capability without presenting another
/// authorization capability. **Warning:** if every initial-admin capability is destroyed (and none
/// was issued separately), the trail is permanently sealed with no admin access. Emits a
/// `CapabilityDestroyed` event on success.
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

/// Transaction wrapper for revoking an initial-admin capability.
///
/// Requires the `RevokeCapabilities` permission. Same denylist semantics as [`WasmRevokeCapability`]
/// but uses the dedicated entry point reserved for initial-admin capability IDs. **Warning:**
/// revoking every initial-admin capability permanently seals the trail. Emits a `CapabilityRevoked`
/// event on success.
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

/// Transaction wrapper for cleaning up expired revoked-capability entries.
///
/// Requires the `RevokeCapabilities` permission. Only prunes denylist entries whose stored
/// `validUntil` is non-zero and strictly less than the current clock time. Entries with
/// `validUntil == 0` are kept indefinitely. Does not revoke additional capabilities. Resolves to a
/// `RevokedCapabilitiesCleanedUp` receipt with `trailId`, `cleanedCount`, `cleanedBy`, and
/// `timestamp`. Emits a `RevokedCapabilitiesCleanedUp` event on success.
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
    ) -> Result<WasmRevokedCapabilitiesCleanedUp> {
        let cleaned: RevokedCapabilitiesCleanedUp =
            apply_with_events(self.0, wasm_effects, wasm_events, client).await?;
        Ok(cleaned.into())
    }
}

/// Transaction wrapper for adding a record.
///
/// Requires the `AddRecord` permission. While the trail's `writeLock` is active the call aborts.
/// Tagged writes additionally require the tag to exist in the trail registry and the supplied
/// capability's role to allow that tag. Records are assigned the trail's current monotonic sequence
/// number, which is never reused even after deletions. Emits a `RecordAdded` event on success.
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

/// Transaction wrapper for deleting a single record.
///
/// Requires the `DeleteRecord` permission. Aborts if no record exists at the supplied sequence
/// number or while the delete-record window still protects it. Tag-aware authorization additionally
/// applies when the record carries a tag. Emits a `RecordDeleted` event on success.
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

/// Transaction wrapper for deleting records in batch form.
///
/// Requires the `DeleteAllRecords` permission. Walks the trail from the front and silently skips
/// records still inside the delete-record window. Tag-aware authorization applies to every record
/// actually deleted. Resolves to the sequence numbers of the records deleted in this batch, in
/// deletion order — at most `limit` entries, possibly fewer. Emits one `RecordDeleted` event per
/// deletion.
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
    ) -> Result<Vec<u64>> {
        apply_with_events(self.0, wasm_effects, wasm_events, client).await
    }
}

/// Transaction wrapper for adding a record tag to the trail registry.
///
/// Requires the `AddRecordTags` permission. Aborts on-chain if the tag is already in the registry.
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

/// Transaction wrapper for removing a record tag from the trail registry.
///
/// Requires the `DeleteRecordTags` permission. Aborts on-chain if the tag is not in the registry or
/// while it is still referenced by any existing record or role-tag restriction.
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
