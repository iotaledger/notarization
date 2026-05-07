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

/// Read-only view of an on-chain audit trail.
///
/// @remarks
/// The trail is a *shared*, tamper-evident object that maintains an ordered sequence of records.
/// Each record is assigned a unique, auto-incrementing sequence number that is never reused (the
/// counter does not decrement on deletion). Access is governed by capability-based RBAC: every
/// mutating call must present a {@link Capability} bound to a role whose permissions cover the
/// operation.
#[wasm_bindgen(js_name = OnChainAuditTrail, inspectable)]
#[derive(Clone)]
pub struct WasmOnChainAuditTrail(pub(crate) OnChainAuditTrail);

#[wasm_bindgen(js_class = OnChainAuditTrail)]
impl WasmOnChainAuditTrail {
    pub(crate) fn new(trail: OnChainAuditTrail) -> Self {
        Self(trail)
    }

    /// Returns the trail object ID.
    ///
    /// @returns Stringified object ID of this trail.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.id.to_string()
    }

    /// Returns the address that created this trail.
    ///
    /// @returns Stringified IOTA address of the trail creator.
    #[wasm_bindgen(getter)]
    pub fn creator(&self) -> String {
        self.0.creator.to_string()
    }

    /// Returns the creation timestamp in milliseconds since the Unix epoch.
    ///
    /// @returns Creation timestamp in milliseconds.
    #[wasm_bindgen(js_name = createdAt, getter)]
    pub fn created_at(&self) -> u64 {
        self.0.created_at
    }

    /// Returns the next sequence number that will be assigned to a new record.
    ///
    /// @remarks
    /// This is a monotonic counter that never decrements, even after records are deleted, so
    /// existing sequence numbers remain unique for the lifetime of the trail.
    ///
    /// @returns Sequence number that the next added record will receive.
    #[wasm_bindgen(js_name = sequenceNumber, getter)]
    pub fn sequence_number(&self) -> u64 {
        self.0.sequence_number
    }

    /// Returns the active locking configuration that governs record deletion, trail deletion, and
    /// record writes.
    ///
    /// @returns Active {@link LockingConfig} for the trail.
    #[wasm_bindgen(js_name = lockingConfig, getter)]
    pub fn locking_config(&self) -> WasmLockingConfig {
        self.0.locking_config.clone().into()
    }

    /// Returns the linked-table metadata for record storage.
    ///
    /// @remarks
    /// Returns table size and head/tail sequence numbers; record contents must be loaded via
    /// {@link TrailRecords}.
    ///
    /// @returns {@link LinkedTable} metadata for the record table.
    #[wasm_bindgen(getter)]
    pub fn records(&self) -> WasmLinkedTable {
        self.0.records.clone().into()
    }

    /// Returns the canonical list of tags that may be attached to records in this trail, together
    /// with their combined usage counts.
    ///
    /// @returns Tag entries sorted alphabetically by tag name.
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

    /// Returns the trail's role definitions, the revoked-capability denylist, and the permissions
    /// required to administer roles and capabilities.
    ///
    /// @returns The trail's {@link RoleMap}.
    #[wasm_bindgen(getter)]
    pub fn roles(&self) -> WasmRoleMap {
        self.0.roles.clone().into()
    }

    /// Returns metadata fixed at creation time, when present.
    ///
    /// @returns The trail's {@link ImmutableMetadata}, or `null` when none was set.
    #[wasm_bindgen(js_name = immutableMetadata, getter)]
    pub fn immutable_metadata(&self) -> Option<WasmImmutableMetadata> {
        self.0.immutable_metadata.clone().map(Into::into)
    }

    /// Returns metadata that holders of {@link Permission.UpdateMetadata} can change after
    /// creation, when present.
    ///
    /// @returns Current value of `updatableMetadata`, or `null` when the field is unset.
    #[wasm_bindgen(js_name = updatableMetadata, getter)]
    pub fn updatable_metadata(&self) -> Option<String> {
        self.0.updatable_metadata.clone()
    }

    /// Returns the on-chain package version of the trail object.
    ///
    /// @remarks
    /// Use {@link AuditTrailHandle.migrate} after a package upgrade if this lags behind the SDK's
    /// expected version.
    ///
    /// @returns Stored package version of the trail object.
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
/// @remarks
/// On execution the audit-trail package shares the new trail object, seeds the reserved
/// {@link RoleMap.initialAdminRoleName | Admin} role, transfers a fresh initial-admin capability to
/// the admin address, and optionally stores the initial record at sequence number `0`, validating
/// its tag against the registry.
///
/// Emits an {@link AuditTrailCreated} event on success.
#[wasm_bindgen(js_name = CreateTrail, inspectable)]
pub struct WasmCreateTrail(pub(crate) CreateTrail);

#[wasm_bindgen(js_class = CreateTrail)]
impl WasmCreateTrail {
    /// Creates a transaction wrapper from an {@link AuditTrailBuilder}.
    ///
    /// @param builder - Fully configured {@link AuditTrailBuilder}.
    #[wasm_bindgen(constructor)]
    pub fn new(builder: WasmAuditTrailBuilder) -> Self {
        Self(CreateTrail::new(builder.0))
    }

    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and then fetches the created trail object.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used to fetch the new trail object.
    ///
    /// @returns The on-chain {@link OnChainAuditTrail} created by the transaction.
    ///
    /// @throws When the expected event is missing or the trail cannot be fetched.
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
/// @remarks
/// Passing `null`/`undefined` for the new metadata clears the `updatableMetadata` field on-chain.
///
/// Requires the {@link Permission.UpdateMetadata} permission.
#[wasm_bindgen(js_name = UpdateMetadata, inspectable)]
pub struct WasmUpdateMetadata(pub(crate) UpdateMetadata);

#[wasm_bindgen(js_class = UpdateMetadata)]
impl WasmUpdateMetadata {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @throws When transaction application fails.
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
/// @remarks
/// Succeeds only when the on-chain trail's package version is strictly less than the package
/// version this binding targets.
///
/// Requires the {@link Permission.Migrate} permission.
#[wasm_bindgen(js_name = Migrate, inspectable)]
pub struct WasmMigrate(pub(crate) Migrate);

#[wasm_bindgen(js_class = Migrate)]
impl WasmMigrate {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @throws When transaction application fails.
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
/// @remarks
/// Aborts on-chain when records still exist or while the configured trail-delete time lock is
/// active.
///
/// Requires the {@link Permission.DeleteAuditTrail} permission.
///
/// Emits an {@link AuditTrailDeleted} event on success.
#[wasm_bindgen(js_name = DeleteAuditTrail, inspectable)]
pub struct WasmDeleteAuditTrail(pub(crate) DeleteAuditTrail);

#[wasm_bindgen(js_class = DeleteAuditTrail)]
impl WasmDeleteAuditTrail {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link AuditTrailDeleted} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// The supplied configuration's `deleteTrailLock` must not be {@link TimeLock.withUntilDestroyed};
/// the call aborts on-chain otherwise.
///
/// Requires the {@link Permission.UpdateLockingConfig} permission.
#[wasm_bindgen(js_name = UpdateLockingConfig, inspectable)]
pub struct WasmUpdateLockingConfig(pub(crate) UpdateLockingConfig);

#[wasm_bindgen(js_class = UpdateLockingConfig)]
impl WasmUpdateLockingConfig {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @throws When transaction application fails.
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
/// @remarks
/// Updates only the rule that locks individual records against deletion (time-based or
/// count-based).
///
/// Requires the {@link Permission.UpdateLockingConfigForDeleteRecord} permission.
#[wasm_bindgen(js_name = UpdateDeleteRecordWindow, inspectable)]
pub struct WasmUpdateDeleteRecordWindow(pub(crate) UpdateDeleteRecordWindow);

#[wasm_bindgen(js_class = UpdateDeleteRecordWindow)]
impl WasmUpdateDeleteRecordWindow {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @throws When transaction application fails.
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
/// @remarks
/// The new lock must not be {@link TimeLock.withUntilDestroyed}.
///
/// Requires the {@link Permission.UpdateLockingConfigForDeleteTrail} permission.
#[wasm_bindgen(js_name = UpdateDeleteTrailLock, inspectable)]
pub struct WasmUpdateDeleteTrailLock(pub(crate) UpdateDeleteTrailLock);

#[wasm_bindgen(js_class = UpdateDeleteTrailLock)]
impl WasmUpdateDeleteTrailLock {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @throws When transaction application fails.
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
/// @remarks
/// While the new lock is active, {@link TrailRecords.add} aborts on-chain.
///
/// Requires the {@link Permission.UpdateLockingConfigForWrite} permission.
#[wasm_bindgen(js_name = UpdateWriteLock, inspectable)]
pub struct WasmUpdateWriteLock(pub(crate) UpdateWriteLock);

#[wasm_bindgen(js_class = UpdateWriteLock)]
impl WasmUpdateWriteLock {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @throws When transaction application fails.
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
/// @remarks
/// Any `roleTags` supplied must already exist in the trail's record-tag registry; the on-chain
/// call aborts otherwise.
///
/// Requires the {@link Permission.AddRoles} permission.
///
/// Emits a {@link RoleCreated} event on success.
#[wasm_bindgen(js_name = CreateRole, inspectable)]
pub struct WasmCreateRole(pub(crate) CreateRole);

#[wasm_bindgen(js_class = CreateRole)]
impl WasmCreateRole {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link RoleCreated} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// Replaces both the role's permissions and its `roleTags`; any newly supplied tag must already be
/// in the trail's record-tag registry.
///
/// Requires the {@link Permission.UpdateRoles} permission.
///
/// Emits a {@link RoleUpdated} event on success.
#[wasm_bindgen(js_name = UpdateRole, inspectable)]
pub struct WasmUpdateRole(pub(crate) UpdateRole);

#[wasm_bindgen(js_class = UpdateRole)]
impl WasmUpdateRole {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link RoleUpdated} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// The reserved initial-admin role (`"Admin"`) cannot be deleted.
///
/// Requires the {@link Permission.DeleteRoles} permission.
///
/// Emits a {@link RoleDeleted} event on success.
#[wasm_bindgen(js_name = DeleteRole, inspectable)]
pub struct WasmDeleteRole(pub(crate) DeleteRole);

#[wasm_bindgen(js_class = DeleteRole)]
impl WasmDeleteRole {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link RoleDeleted} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// Mints a new {@link Capability} for the role and transfers it to the configured recipient (or
/// the caller when none was set). The validity window configured via
/// {@link CapabilityIssueOptions} is enforced when the capability is later presented for
/// authorization.
///
/// Requires the {@link Permission.AddCapabilities} permission.
///
/// Emits a {@link CapabilityIssued} event on success.
#[wasm_bindgen(js_name = IssueCapability, inspectable)]
pub struct WasmIssueCapability(pub(crate) IssueCapability);

#[wasm_bindgen(js_class = IssueCapability)]
impl WasmIssueCapability {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link CapabilityIssued} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// Adds the capability ID to the trail's denylist. Pass `capabilityValidUntil` so
/// {@link CleanupRevokedCapabilities} can later prune the entry once that timestamp elapses; pass
/// `null` to keep the denylist entry permanently.
///
/// Requires the {@link Permission.RevokeCapabilities} permission.
///
/// Emits a {@link CapabilityRevoked} event on success.
#[wasm_bindgen(js_name = RevokeCapability, inspectable)]
pub struct WasmRevokeCapability(pub(crate) RevokeCapability);

#[wasm_bindgen(js_class = RevokeCapability)]
impl WasmRevokeCapability {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link CapabilityRevoked} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// Consumes the owned capability object. This path is for ordinary capabilities only —
/// initial-admin capabilities must use {@link DestroyInitialAdminCapability}.
///
/// Requires the {@link Permission.RevokeCapabilities} permission.
///
/// Emits a {@link CapabilityDestroyed} event on success.
#[wasm_bindgen(js_name = DestroyCapability, inspectable)]
pub struct WasmDestroyCapability(pub(crate) DestroyCapability);

#[wasm_bindgen(js_class = DestroyCapability)]
impl WasmDestroyCapability {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link CapabilityDestroyed} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// Self-service: the holder consumes their own initial-admin capability without presenting another
/// authorization capability. **Warning:** if every initial-admin capability is destroyed (and none
/// was issued separately), the trail is permanently sealed with no admin access.
///
/// Emits a {@link CapabilityDestroyed} event on success.
#[wasm_bindgen(js_name = DestroyInitialAdminCapability, inspectable)]
pub struct WasmDestroyInitialAdminCapability(pub(crate) DestroyInitialAdminCapability);

#[wasm_bindgen(js_class = DestroyInitialAdminCapability)]
impl WasmDestroyInitialAdminCapability {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link CapabilityDestroyed} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// Same denylist semantics as {@link RevokeCapability} but uses the dedicated entry point reserved
/// for initial-admin capability IDs. **Warning:** revoking every initial-admin capability
/// permanently seals the trail.
///
/// Requires the {@link Permission.RevokeCapabilities} permission.
///
/// Emits a {@link CapabilityRevoked} event on success.
#[wasm_bindgen(js_name = RevokeInitialAdminCapability, inspectable)]
pub struct WasmRevokeInitialAdminCapability(pub(crate) RevokeInitialAdminCapability);

#[wasm_bindgen(js_class = RevokeInitialAdminCapability)]
impl WasmRevokeInitialAdminCapability {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link CapabilityRevoked} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// Only prunes denylist entries whose stored `validUntil` is non-zero and strictly less than the
/// current clock time. Entries with `validUntil == 0` are kept indefinitely. Does not revoke
/// additional capabilities.
///
/// Requires the {@link Permission.RevokeCapabilities} permission.
///
/// Emits a {@link RevokedCapabilitiesCleanedUp} event on success.
#[wasm_bindgen(js_name = CleanupRevokedCapabilities, inspectable)]
pub struct WasmCleanupRevokedCapabilities(pub(crate) CleanupRevokedCapabilities);

#[wasm_bindgen(js_class = CleanupRevokedCapabilities)]
impl WasmCleanupRevokedCapabilities {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link RevokedCapabilitiesCleanedUp} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// While the trail's `writeLock` is active the call aborts. Tagged writes additionally require the
/// tag to exist in the trail registry and the supplied capability's role to allow that tag.
/// Records are assigned the trail's current monotonic sequence number, which is never reused even
/// after deletions.
///
/// Requires the {@link Permission.AddRecord} permission.
///
/// Emits a {@link RecordAdded} event on success.
#[wasm_bindgen(js_name = AddRecord, inspectable)]
pub struct WasmAddRecord(pub(crate) AddRecord);

#[wasm_bindgen(js_class = AddRecord)]
impl WasmAddRecord {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link RecordAdded} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// Aborts on-chain when no record exists at the supplied sequence number or while the
/// delete-record window still protects it. Tag-aware authorization additionally applies when the
/// record carries a tag.
///
/// Requires the {@link Permission.DeleteRecord} permission.
///
/// Emits a {@link RecordDeleted} event on success.
#[wasm_bindgen(js_name = DeleteRecord, inspectable)]
pub struct WasmDeleteRecord(pub(crate) DeleteRecord);

#[wasm_bindgen(js_class = DeleteRecord)]
impl WasmDeleteRecord {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching event payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Decoded {@link RecordDeleted} event payload.
    ///
    /// @throws When the expected event is missing or transaction application fails.
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
/// @remarks
/// Walks the trail from the front and silently skips records still inside the delete-record
/// window. Tag-aware authorization applies to every record actually deleted.
///
/// Requires the {@link Permission.DeleteAllRecords} permission.
///
/// Emits one {@link RecordDeleted} event per deletion.
#[wasm_bindgen(js_name = DeleteRecordsBatch, inspectable)]
pub struct WasmDeleteRecordsBatch(pub(crate) DeleteRecordsBatch);

#[wasm_bindgen(js_class = DeleteRecordsBatch)]
impl WasmDeleteRecordsBatch {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events and decodes the matching payload.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @returns Sequence numbers of the records deleted in this batch, in deletion order — at
    /// most the requested limit, possibly fewer.
    ///
    /// @throws When transaction application fails.
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
/// @remarks
/// Aborts on-chain if the tag is already in the registry.
///
/// Requires the {@link Permission.AddRecordTags} permission.
#[wasm_bindgen(js_name = AddRecordTag, inspectable)]
pub struct WasmAddRecordTag(pub(crate) AddRecordTag);

#[wasm_bindgen(js_class = AddRecordTag)]
impl WasmAddRecordTag {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @throws When transaction application fails.
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
/// @remarks
/// Aborts on-chain if the tag is not in the registry or while it is still referenced by any
/// existing record or role-tag restriction.
///
/// Requires the {@link Permission.DeleteRecordTags} permission.
#[wasm_bindgen(js_name = RemoveRecordTag, inspectable)]
pub struct WasmRemoveRecordTag(pub(crate) RemoveRecordTag);

#[wasm_bindgen(js_class = RemoveRecordTag)]
impl WasmRemoveRecordTag {
    /// Builds the programmable transaction bytes for submission.
    ///
    /// @param client - Read-only core client used to resolve packages and serialize the
    /// transaction.
    ///
    /// @returns BCS-encoded programmable transaction bytes ready for signing and submission.
    ///
    /// @throws When transaction serialization fails.
    #[wasm_bindgen(js_name = buildProgrammableTransaction)]
    pub async fn build_programmable_transaction(&self, client: &WasmCoreClientReadOnly) -> Result<Vec<u8>> {
        build_programmable_transaction(&self.0, client).await
    }

    /// Applies transaction effects and events.
    ///
    /// @param wasmEffects - Effects of the executed transaction.
    /// @param wasmEvents - Events emitted by the executed transaction.
    /// @param client - Read-only core client used during application.
    ///
    /// @throws When transaction application fails.
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
