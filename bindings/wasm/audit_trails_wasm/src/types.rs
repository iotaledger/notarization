// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use audit_trails::core::types::{
    AuditTrailCreated, AuditTrailDeleted, Capability, CapabilityAdminPermissions, CapabilityDestroyed,
    CapabilityIssueOptions, CapabilityIssued, CapabilityRevoked, Data, ImmutableMetadata, LockingConfig, LockingWindow,
    PaginatedRecord, Permission, PermissionSet, Record, RecordAdded, RecordCorrection, RecordDeleted,
    RoleAdminPermissions, RoleCreated, RoleMap, RoleRemoved, RoleUpdated, TimeLock,
};
use iota_interaction::types::collection_types::LinkedTable;
use js_sys::Uint8Array;
use product_common::bindings::WasmIotaAddress;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Empty, inspectable)]
pub struct WasmEmpty;

impl From<()> for WasmEmpty {
    fn from(_: ()) -> Self {
        Self
    }
}

#[wasm_bindgen(js_name = Data, inspectable)]
#[derive(Clone)]
pub struct WasmData(pub(crate) Data);

#[wasm_bindgen(js_class = Data)]
impl WasmData {
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> JsValue {
        match &self.0 {
            Data::Bytes(bytes) => Uint8Array::from(bytes.as_slice()).into(),
            Data::Text(text) => JsValue::from(text),
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        match &self.0 {
            Data::Bytes(bytes) => String::from_utf8_lossy(bytes).to_string(),
            Data::Text(text) => text.clone(),
        }
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        match &self.0 {
            Data::Bytes(bytes) => bytes.clone(),
            Data::Text(text) => text.as_bytes().to_vec(),
        }
    }

    #[wasm_bindgen(js_name = fromString)]
    pub fn from_string(data: String) -> Self {
        Self(Data::text(data))
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(data: Uint8Array) -> Self {
        Self(Data::bytes(data.to_vec()))
    }
}

impl From<Data> for WasmData {
    fn from(value: Data) -> Self {
        Self(value)
    }
}

impl From<WasmData> for Data {
    fn from(value: WasmData) -> Self {
        value.0
    }
}

fn permission_sort_key(permission: Permission) -> u8 {
    match permission {
        Permission::DeleteAuditTrail => 0,
        Permission::DeleteAllRecords => 1,
        Permission::AddRecord => 2,
        Permission::DeleteRecord => 3,
        Permission::CorrectRecord => 4,
        Permission::UpdateLockingConfig => 5,
        Permission::UpdateLockingConfigForDeleteRecord => 6,
        Permission::UpdateLockingConfigForDeleteTrail => 7,
        Permission::UpdateLockingConfigForWrite => 8,
        Permission::AddRoles => 9,
        Permission::UpdateRoles => 10,
        Permission::DeleteRoles => 11,
        Permission::AddCapabilities => 12,
        Permission::RevokeCapabilities => 13,
        Permission::UpdateMetadata => 14,
        Permission::DeleteMetadata => 15,
        Permission::Migrate => 16,
    }
}

fn sorted_permissions_from_set(permissions: HashSet<Permission>) -> Vec<WasmPermission> {
    let mut permissions: Vec<_> = permissions.into_iter().collect();
    permissions.sort_unstable_by_key(|permission| permission_sort_key(*permission));
    permissions.into_iter().map(Into::into).collect()
}

fn sorted_object_ids(ids: HashSet<iota_interaction::types::base_types::ObjectID>) -> Vec<String> {
    let mut ids: Vec<_> = ids.into_iter().map(|id| id.to_string()).collect();
    ids.sort_unstable();
    ids
}

fn sorted_role_entries(roles: HashMap<String, HashSet<Permission>>) -> Vec<WasmRolePermissionsEntry> {
    let mut roles: Vec<_> = roles
        .into_iter()
        .map(|(name, permissions)| WasmRolePermissionsEntry {
            name,
            permissions: sorted_permissions_from_set(permissions),
        })
        .collect();
    roles.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    roles
}

#[wasm_bindgen(js_name = Permission)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmPermission {
    DeleteAuditTrail,
    DeleteAllRecords,
    AddRecord,
    DeleteRecord,
    CorrectRecord,
    UpdateLockingConfig,
    UpdateLockingConfigForDeleteRecord,
    UpdateLockingConfigForDeleteTrail,
    UpdateLockingConfigForWrite,
    AddRoles,
    UpdateRoles,
    DeleteRoles,
    AddCapabilities,
    RevokeCapabilities,
    UpdateMetadata,
    DeleteMetadata,
    Migrate,
}

impl From<Permission> for WasmPermission {
    fn from(value: Permission) -> Self {
        match value {
            Permission::DeleteAuditTrail => Self::DeleteAuditTrail,
            Permission::DeleteAllRecords => Self::DeleteAllRecords,
            Permission::AddRecord => Self::AddRecord,
            Permission::DeleteRecord => Self::DeleteRecord,
            Permission::CorrectRecord => Self::CorrectRecord,
            Permission::UpdateLockingConfig => Self::UpdateLockingConfig,
            Permission::UpdateLockingConfigForDeleteRecord => Self::UpdateLockingConfigForDeleteRecord,
            Permission::UpdateLockingConfigForDeleteTrail => Self::UpdateLockingConfigForDeleteTrail,
            Permission::UpdateLockingConfigForWrite => Self::UpdateLockingConfigForWrite,
            Permission::AddRoles => Self::AddRoles,
            Permission::UpdateRoles => Self::UpdateRoles,
            Permission::DeleteRoles => Self::DeleteRoles,
            Permission::AddCapabilities => Self::AddCapabilities,
            Permission::RevokeCapabilities => Self::RevokeCapabilities,
            Permission::UpdateMetadata => Self::UpdateMetadata,
            Permission::DeleteMetadata => Self::DeleteMetadata,
            Permission::Migrate => Self::Migrate,
        }
    }
}

impl From<WasmPermission> for Permission {
    fn from(value: WasmPermission) -> Self {
        match value {
            WasmPermission::DeleteAuditTrail => Self::DeleteAuditTrail,
            WasmPermission::DeleteAllRecords => Self::DeleteAllRecords,
            WasmPermission::AddRecord => Self::AddRecord,
            WasmPermission::DeleteRecord => Self::DeleteRecord,
            WasmPermission::CorrectRecord => Self::CorrectRecord,
            WasmPermission::UpdateLockingConfig => Self::UpdateLockingConfig,
            WasmPermission::UpdateLockingConfigForDeleteRecord => Self::UpdateLockingConfigForDeleteRecord,
            WasmPermission::UpdateLockingConfigForDeleteTrail => Self::UpdateLockingConfigForDeleteTrail,
            WasmPermission::UpdateLockingConfigForWrite => Self::UpdateLockingConfigForWrite,
            WasmPermission::AddRoles => Self::AddRoles,
            WasmPermission::UpdateRoles => Self::UpdateRoles,
            WasmPermission::DeleteRoles => Self::DeleteRoles,
            WasmPermission::AddCapabilities => Self::AddCapabilities,
            WasmPermission::RevokeCapabilities => Self::RevokeCapabilities,
            WasmPermission::UpdateMetadata => Self::UpdateMetadata,
            WasmPermission::DeleteMetadata => Self::DeleteMetadata,
            WasmPermission::Migrate => Self::Migrate,
        }
    }
}

#[wasm_bindgen(js_name = PermissionSet, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmPermissionSet {
    pub permissions: Vec<WasmPermission>,
}

#[wasm_bindgen(js_class = PermissionSet)]
impl WasmPermissionSet {
    #[wasm_bindgen(constructor)]
    pub fn new(permissions: Vec<WasmPermission>) -> Self {
        Self { permissions }
    }

    #[wasm_bindgen(js_name = adminPermissions)]
    pub fn admin_permissions() -> Self {
        PermissionSet::admin_permissions().into()
    }

    #[wasm_bindgen(js_name = recordAdminPermissions)]
    pub fn record_admin_permissions() -> Self {
        PermissionSet::record_admin_permissions().into()
    }

    #[wasm_bindgen(js_name = lockingAdminPermissions)]
    pub fn locking_admin_permissions() -> Self {
        PermissionSet::locking_admin_permissions().into()
    }

    #[wasm_bindgen(js_name = roleAdminPermissions)]
    pub fn role_admin_permissions() -> Self {
        PermissionSet::role_admin_permissions().into()
    }

    #[wasm_bindgen(js_name = capAdminPermissions)]
    pub fn cap_admin_permissions() -> Self {
        PermissionSet::cap_admin_permissions().into()
    }

    #[wasm_bindgen(js_name = metadataAdminPermissions)]
    pub fn metadata_admin_permissions() -> Self {
        PermissionSet::metadata_admin_permissions().into()
    }
}

impl From<PermissionSet> for WasmPermissionSet {
    fn from(value: PermissionSet) -> Self {
        Self {
            permissions: sorted_permissions_from_set(value.permissions),
        }
    }
}

impl From<WasmPermissionSet> for PermissionSet {
    fn from(value: WasmPermissionSet) -> Self {
        Self {
            permissions: value.permissions.into_iter().map(Into::into).collect(),
        }
    }
}

#[wasm_bindgen(js_name = LinkedTable, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmLinkedTable {
    pub id: String,
    pub size: u64,
    pub head: Option<u64>,
    pub tail: Option<u64>,
}

impl From<LinkedTable<u64>> for WasmLinkedTable {
    fn from(value: LinkedTable<u64>) -> Self {
        Self {
            id: value.id.to_string(),
            size: value.size,
            head: value.head,
            tail: value.tail,
        }
    }
}

#[wasm_bindgen(js_name = RoleAdminPermissions, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmRoleAdminPermissions {
    pub add: WasmPermission,
    pub delete: WasmPermission,
    pub update: WasmPermission,
}

impl From<RoleAdminPermissions> for WasmRoleAdminPermissions {
    fn from(value: RoleAdminPermissions) -> Self {
        Self {
            add: value.add.into(),
            delete: value.delete.into(),
            update: value.update.into(),
        }
    }
}

#[wasm_bindgen(js_name = CapabilityAdminPermissions, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmCapabilityAdminPermissions {
    pub add: WasmPermission,
    pub revoke: WasmPermission,
}

impl From<CapabilityAdminPermissions> for WasmCapabilityAdminPermissions {
    fn from(value: CapabilityAdminPermissions) -> Self {
        Self {
            add: value.add.into(),
            revoke: value.revoke.into(),
        }
    }
}

#[wasm_bindgen(js_name = RolePermissionsEntry, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRolePermissionsEntry {
    pub name: String,
    pub permissions: Vec<WasmPermission>,
}

#[wasm_bindgen(js_name = RoleMap, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRoleMap {
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    pub roles: Vec<WasmRolePermissionsEntry>,
    #[wasm_bindgen(js_name = initialAdminRoleName)]
    pub initial_admin_role_name: String,
    #[wasm_bindgen(js_name = issuedCapabilities)]
    pub issued_capabilities: Vec<String>,
    #[wasm_bindgen(js_name = initialAdminCapIds)]
    pub initial_admin_cap_ids: Vec<String>,
    #[wasm_bindgen(js_name = roleAdminPermissions)]
    pub role_admin_permissions: WasmRoleAdminPermissions,
    #[wasm_bindgen(js_name = capabilityAdminPermissions)]
    pub capability_admin_permissions: WasmCapabilityAdminPermissions,
}

impl From<RoleMap> for WasmRoleMap {
    fn from(value: RoleMap) -> Self {
        Self {
            target_key: value.target_key.to_string(),
            roles: sorted_role_entries(value.roles),
            initial_admin_role_name: value.initial_admin_role_name,
            issued_capabilities: sorted_object_ids(value.issued_capabilities),
            initial_admin_cap_ids: sorted_object_ids(value.initial_admin_cap_ids),
            role_admin_permissions: value.role_admin_permissions.into(),
            capability_admin_permissions: value.capability_admin_permissions.into(),
        }
    }
}

#[wasm_bindgen(js_name = CapabilityIssueOptions, getter_with_clone, inspectable)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct WasmCapabilityIssueOptions {
    #[wasm_bindgen(js_name = issuedTo)]
    pub issued_to: Option<WasmIotaAddress>,
    #[wasm_bindgen(js_name = validFromMs)]
    pub valid_from_ms: Option<u64>,
    #[wasm_bindgen(js_name = validUntilMs)]
    pub valid_until_ms: Option<u64>,
}

#[wasm_bindgen(js_class = CapabilityIssueOptions)]
impl WasmCapabilityIssueOptions {
    #[wasm_bindgen(constructor)]
    pub fn new(issued_to: Option<WasmIotaAddress>, valid_from_ms: Option<u64>, valid_until_ms: Option<u64>) -> Self {
        Self {
            issued_to,
            valid_from_ms,
            valid_until_ms,
        }
    }
}

impl From<CapabilityIssueOptions> for WasmCapabilityIssueOptions {
    fn from(value: CapabilityIssueOptions) -> Self {
        Self {
            issued_to: value.issued_to.map(|address| address.to_string()),
            valid_from_ms: value.valid_from_ms,
            valid_until_ms: value.valid_until_ms,
        }
    }
}

impl From<WasmCapabilityIssueOptions> for CapabilityIssueOptions {
    fn from(value: WasmCapabilityIssueOptions) -> Self {
        Self {
            issued_to: value.issued_to.and_then(|address| address.parse().ok()),
            valid_from_ms: value.valid_from_ms,
            valid_until_ms: value.valid_until_ms,
        }
    }
}

#[wasm_bindgen(js_name = Capability, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmCapability {
    pub id: String,
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    pub role: String,
    #[wasm_bindgen(js_name = issuedTo)]
    pub issued_to: Option<WasmIotaAddress>,
    #[wasm_bindgen(js_name = validFrom)]
    pub valid_from: Option<u64>,
    #[wasm_bindgen(js_name = validUntil)]
    pub valid_until: Option<u64>,
}

impl From<Capability> for WasmCapability {
    fn from(value: Capability) -> Self {
        Self {
            id: value.id.id.to_string(),
            target_key: value.target_key.to_string(),
            role: value.role,
            issued_to: value.issued_to.map(|address| address.to_string()),
            valid_from: value.valid_from,
            valid_until: value.valid_until,
        }
    }
}

#[wasm_bindgen(js_name = AuditTrailCreated, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmAuditTrailCreated {
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    pub creator: WasmIotaAddress,
    pub timestamp: u64,
}

impl From<AuditTrailCreated> for WasmAuditTrailCreated {
    fn from(value: AuditTrailCreated) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            creator: value.creator.to_string(),
            timestamp: value.timestamp,
        }
    }
}

#[wasm_bindgen(js_name = AuditTrailDeleted, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmAuditTrailDeleted {
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    pub timestamp: u64,
}

impl From<AuditTrailDeleted> for WasmAuditTrailDeleted {
    fn from(value: AuditTrailDeleted) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            timestamp: value.timestamp,
        }
    }
}

#[wasm_bindgen(js_name = RecordAdded, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRecordAdded {
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    #[wasm_bindgen(js_name = sequenceNumber)]
    pub sequence_number: u64,
    #[wasm_bindgen(js_name = addedBy)]
    pub added_by: WasmIotaAddress,
    pub timestamp: u64,
}

impl From<RecordAdded> for WasmRecordAdded {
    fn from(value: RecordAdded) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            sequence_number: value.sequence_number,
            added_by: value.added_by.to_string(),
            timestamp: value.timestamp,
        }
    }
}

#[wasm_bindgen(js_name = RecordDeleted, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRecordDeleted {
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    #[wasm_bindgen(js_name = sequenceNumber)]
    pub sequence_number: u64,
    #[wasm_bindgen(js_name = deletedBy)]
    pub deleted_by: WasmIotaAddress,
    pub timestamp: u64,
}

impl From<RecordDeleted> for WasmRecordDeleted {
    fn from(value: RecordDeleted) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            sequence_number: value.sequence_number,
            deleted_by: value.deleted_by.to_string(),
            timestamp: value.timestamp,
        }
    }
}

#[wasm_bindgen(js_name = CapabilityIssued, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmCapabilityIssued {
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    #[wasm_bindgen(js_name = capabilityId)]
    pub capability_id: String,
    pub role: String,
    #[wasm_bindgen(js_name = issuedTo)]
    pub issued_to: Option<WasmIotaAddress>,
    #[wasm_bindgen(js_name = validFrom)]
    pub valid_from: Option<u64>,
    #[wasm_bindgen(js_name = validUntil)]
    pub valid_until: Option<u64>,
}

impl From<CapabilityIssued> for WasmCapabilityIssued {
    fn from(value: CapabilityIssued) -> Self {
        Self {
            target_key: value.target_key.to_string(),
            capability_id: value.capability_id.to_string(),
            role: value.role,
            issued_to: value.issued_to.map(|address| address.to_string()),
            valid_from: value.valid_from,
            valid_until: value.valid_until,
        }
    }
}

#[wasm_bindgen(js_name = CapabilityDestroyed, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmCapabilityDestroyed {
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    #[wasm_bindgen(js_name = capabilityId)]
    pub capability_id: String,
}

impl From<CapabilityDestroyed> for WasmCapabilityDestroyed {
    fn from(value: CapabilityDestroyed) -> Self {
        Self {
            target_key: value.target_key.to_string(),
            capability_id: value.capability_id.to_string(),
        }
    }
}

#[wasm_bindgen(js_name = CapabilityRevoked, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmCapabilityRevoked {
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    #[wasm_bindgen(js_name = capabilityId)]
    pub capability_id: String,
}

impl From<CapabilityRevoked> for WasmCapabilityRevoked {
    fn from(value: CapabilityRevoked) -> Self {
        Self {
            target_key: value.target_key.to_string(),
            capability_id: value.capability_id.to_string(),
        }
    }
}

#[wasm_bindgen(js_name = RoleCreated, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRoleCreated {
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    pub role: String,
}

impl From<RoleCreated> for WasmRoleCreated {
    fn from(value: RoleCreated) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            role: value.role,
        }
    }
}

#[wasm_bindgen(js_name = RoleUpdated, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRoleUpdated {
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    pub role: String,
}

impl From<RoleUpdated> for WasmRoleUpdated {
    fn from(value: RoleUpdated) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            role: value.role,
        }
    }
}

#[wasm_bindgen(js_name = RoleRemoved, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRoleRemoved {
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    pub role: String,
}

impl From<RoleRemoved> for WasmRoleRemoved {
    fn from(value: RoleRemoved) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            role: value.role,
        }
    }
}

#[wasm_bindgen(js_name = TimeLockType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmTimeLockType {
    None,
    UnlockAt,
    UnlockAtMs,
    UntilDestroyed,
    Infinite,
}

#[wasm_bindgen(js_name = TimeLock, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTimeLock(pub(crate) TimeLock);

#[wasm_bindgen(js_class = TimeLock)]
impl WasmTimeLock {
    #[wasm_bindgen(js_name = withUnlockAt)]
    pub fn with_unlock_at(time_sec: u32) -> Self {
        Self(TimeLock::UnlockAt(time_sec))
    }

    #[wasm_bindgen(js_name = withUnlockAtMs)]
    pub fn with_unlock_at_ms(time_ms: u64) -> Self {
        Self(TimeLock::UnlockAtMs(time_ms))
    }

    #[wasm_bindgen(js_name = withUntilDestroyed)]
    pub fn with_until_destroyed() -> Self {
        Self(TimeLock::UntilDestroyed)
    }

    #[wasm_bindgen(js_name = withInfinite)]
    pub fn with_infinite() -> Self {
        Self(TimeLock::Infinite)
    }

    #[wasm_bindgen(js_name = withNone)]
    pub fn with_none() -> Self {
        Self(TimeLock::None)
    }

    #[wasm_bindgen(js_name = "type", getter)]
    pub fn lock_type(&self) -> WasmTimeLockType {
        match self.0 {
            TimeLock::None => WasmTimeLockType::None,
            TimeLock::UnlockAt(_) => WasmTimeLockType::UnlockAt,
            TimeLock::UnlockAtMs(_) => WasmTimeLockType::UnlockAtMs,
            TimeLock::UntilDestroyed => WasmTimeLockType::UntilDestroyed,
            TimeLock::Infinite => WasmTimeLockType::Infinite,
        }
    }

    #[wasm_bindgen(js_name = "args", getter)]
    pub fn args(&self) -> JsValue {
        match self.0 {
            TimeLock::UnlockAt(value) => JsValue::from(value),
            TimeLock::UnlockAtMs(value) => JsValue::from(value),
            _ => JsValue::UNDEFINED,
        }
    }
}

impl From<TimeLock> for WasmTimeLock {
    fn from(value: TimeLock) -> Self {
        Self(value)
    }
}

impl From<WasmTimeLock> for TimeLock {
    fn from(value: WasmTimeLock) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_name = LockingWindowType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmLockingWindowType {
    None,
    TimeBased,
    CountBased,
}

#[wasm_bindgen(js_name = LockingWindow, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmLockingWindow(pub(crate) LockingWindow);

#[wasm_bindgen(js_class = LockingWindow)]
impl WasmLockingWindow {
    #[wasm_bindgen(js_name = withNone)]
    pub fn with_none() -> Self {
        Self(LockingWindow::None)
    }

    #[wasm_bindgen(js_name = withTimeBased)]
    pub fn with_time_based(seconds: u64) -> Self {
        Self(LockingWindow::TimeBased { seconds })
    }

    #[wasm_bindgen(js_name = withCountBased)]
    pub fn with_count_based(count: u64) -> Self {
        Self(LockingWindow::CountBased { count })
    }

    #[wasm_bindgen(js_name = "type", getter)]
    pub fn window_type(&self) -> WasmLockingWindowType {
        match self.0 {
            LockingWindow::None => WasmLockingWindowType::None,
            LockingWindow::TimeBased { .. } => WasmLockingWindowType::TimeBased,
            LockingWindow::CountBased { .. } => WasmLockingWindowType::CountBased,
        }
    }

    #[wasm_bindgen(js_name = "args", getter)]
    pub fn args(&self) -> JsValue {
        match self.0 {
            LockingWindow::TimeBased { seconds } => JsValue::from(seconds),
            LockingWindow::CountBased { count } => JsValue::from(count),
            LockingWindow::None => JsValue::UNDEFINED,
        }
    }
}

impl From<LockingWindow> for WasmLockingWindow {
    fn from(value: LockingWindow) -> Self {
        Self(value)
    }
}

impl From<WasmLockingWindow> for LockingWindow {
    fn from(value: WasmLockingWindow) -> Self {
        value.0
    }
}

#[wasm_bindgen(js_name = LockingConfig, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmLockingConfig {
    #[wasm_bindgen(js_name = deleteRecordWindow)]
    pub delete_record_window: WasmLockingWindow,
    #[wasm_bindgen(js_name = deleteTrailLock)]
    pub delete_trail_lock: WasmTimeLock,
    #[wasm_bindgen(js_name = writeLock)]
    pub write_lock: WasmTimeLock,
}

#[wasm_bindgen(js_class = LockingConfig)]
impl WasmLockingConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(
        delete_record_window: WasmLockingWindow,
        delete_trail_lock: WasmTimeLock,
        write_lock: WasmTimeLock,
    ) -> Self {
        Self {
            delete_record_window,
            delete_trail_lock,
            write_lock,
        }
    }
}

impl From<LockingConfig> for WasmLockingConfig {
    fn from(value: LockingConfig) -> Self {
        Self {
            delete_record_window: value.delete_record_window.into(),
            delete_trail_lock: value.delete_trail_lock.into(),
            write_lock: value.write_lock.into(),
        }
    }
}

impl From<WasmLockingConfig> for LockingConfig {
    fn from(value: WasmLockingConfig) -> Self {
        Self {
            delete_record_window: value.delete_record_window.into(),
            delete_trail_lock: value.delete_trail_lock.into(),
            write_lock: value.write_lock.into(),
        }
    }
}

#[wasm_bindgen(js_name = ImmutableMetadata, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmImmutableMetadata {
    pub name: String,
    pub description: Option<String>,
}

impl From<ImmutableMetadata> for WasmImmutableMetadata {
    fn from(value: ImmutableMetadata) -> Self {
        Self {
            name: value.name,
            description: value.description,
        }
    }
}

impl From<WasmImmutableMetadata> for ImmutableMetadata {
    fn from(value: WasmImmutableMetadata) -> Self {
        ImmutableMetadata {
            name: value.name,
            description: value.description,
        }
    }
}

#[wasm_bindgen(js_name = RecordCorrection, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmRecordCorrection {
    pub replaces: Vec<u64>,
    #[wasm_bindgen(js_name = isReplacedBy)]
    pub is_replaced_by: Option<u64>,
}

impl From<RecordCorrection> for WasmRecordCorrection {
    fn from(value: RecordCorrection) -> Self {
        let mut replaces: Vec<u64> = value.replaces.into_iter().collect();
        replaces.sort_unstable();
        Self {
            replaces,
            is_replaced_by: value.is_replaced_by,
        }
    }
}

impl From<WasmRecordCorrection> for RecordCorrection {
    fn from(value: WasmRecordCorrection) -> Self {
        Self {
            replaces: value.replaces.into_iter().collect::<HashSet<_>>(),
            is_replaced_by: value.is_replaced_by,
        }
    }
}

#[wasm_bindgen(js_name = Record, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRecord {
    pub data: WasmData,
    pub metadata: Option<String>,
    #[wasm_bindgen(js_name = sequenceNumber)]
    pub sequence_number: u64,
    #[wasm_bindgen(js_name = addedBy)]
    pub added_by: WasmIotaAddress,
    #[wasm_bindgen(js_name = addedAt)]
    pub added_at: u64,
    pub correction: WasmRecordCorrection,
}

impl From<Record<Data>> for WasmRecord {
    fn from(value: Record<Data>) -> Self {
        Self {
            data: value.data.into(),
            metadata: value.metadata,
            sequence_number: value.sequence_number,
            added_by: value.added_by.to_string(),
            added_at: value.added_at,
            correction: value.correction.into(),
        }
    }
}

#[wasm_bindgen(js_name = PaginatedRecord, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmPaginatedRecord {
    pub records: Vec<WasmRecord>,
    #[wasm_bindgen(js_name = nextCursor)]
    pub next_cursor: Option<u64>,
    #[wasm_bindgen(js_name = hasNextPage)]
    pub has_next_page: bool,
}

impl From<PaginatedRecord<Data>> for WasmPaginatedRecord {
    fn from(value: PaginatedRecord<Data>) -> Self {
        Self {
            records: value.records.into_values().map(Into::into).collect(),
            next_cursor: value.next_cursor,
            has_next_page: value.has_next_page,
        }
    }
}
