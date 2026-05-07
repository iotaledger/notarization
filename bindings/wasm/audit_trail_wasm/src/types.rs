// Copyright 2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use audit_trail::core::types::{
    AuditTrailCreated, AuditTrailDeleted, Capability, CapabilityAdminPermissions, CapabilityDestroyed,
    CapabilityIssueOptions, CapabilityIssued, CapabilityRevoked, Data, ImmutableMetadata, LockingConfig, LockingWindow,
    PaginatedRecord, Permission, PermissionSet, Record, RecordAdded, RecordCorrection, RecordDeleted,
    RevokedCapabilitiesCleanedUp, Role, RoleAdminPermissions, RoleCreated, RoleDeleted, RoleMap, RoleTags, RoleUpdated,
    TimeLock,
};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::collection_types::LinkedTable;
use js_sys::Uint8Array;
use product_common::bindings::WasmIotaAddress;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Placeholder type used as the resolved value of transactions that carry no payload.
#[wasm_bindgen(js_name = Empty, inspectable)]
pub struct WasmEmpty;

impl From<()> for WasmEmpty {
    fn from(_: ()) -> Self {
        Self
    }
}

/// Audit-trail record payload.
///
/// @remarks
/// Holds either a UTF-8 string or a raw byte sequence. Use {@link Data.fromString} or
/// {@link Data.fromBytes} to construct an instance, and {@link Data.toString} or
/// {@link Data.toBytes} to extract the payload as the desired representation.
#[wasm_bindgen(js_name = Data, inspectable)]
#[derive(Clone)]
pub struct WasmData(pub(crate) Data);

#[wasm_bindgen(js_class = Data)]
impl WasmData {
    /// Returns the underlying payload in its original representation.
    ///
    /// @returns A `string` for text payloads or a `Uint8Array` for byte payloads.
    #[wasm_bindgen(getter)]
    pub fn value(&self) -> JsValue {
        match &self.0 {
            Data::Bytes(bytes) => Uint8Array::from(bytes.as_slice()).into(),
            Data::Text(text) => JsValue::from(text),
        }
    }

    /// Returns the payload as a string.
    ///
    /// @remarks
    /// Byte payloads are decoded with lossy UTF-8 conversion (invalid sequences become the U+FFFD
    /// replacement character).
    ///
    /// @returns A string view of the payload.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        match &self.0 {
            Data::Bytes(bytes) => String::from_utf8_lossy(bytes).to_string(),
            Data::Text(text) => text.clone(),
        }
    }

    /// Returns the payload as raw bytes.
    ///
    /// @remarks
    /// Text payloads are encoded as UTF-8.
    ///
    /// @returns A byte view of the payload.
    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        match &self.0 {
            Data::Bytes(bytes) => bytes.clone(),
            Data::Text(text) => text.as_bytes().to_vec(),
        }
    }

    /// Creates a text payload.
    ///
    /// @param data - UTF-8 string to wrap.
    ///
    /// @returns A {@link Data} carrying `data` as text.
    #[wasm_bindgen(js_name = fromString)]
    pub fn from_string(data: String) -> Self {
        Self(Data::text(data))
    }

    /// Creates a binary payload.
    ///
    /// @param data - Raw bytes to wrap.
    ///
    /// @returns A {@link Data} carrying `data` as bytes.
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
        Permission::AddRecordTags => 17,
        Permission::DeleteRecordTags => 18,
    }
}

fn sorted_permissions_from_set(permissions: HashSet<Permission>) -> Vec<WasmPermission> {
    let mut permissions: Vec<_> = permissions.into_iter().collect();
    permissions.sort_unstable_by_key(|permission| permission_sort_key(*permission));
    permissions.into_iter().map(Into::into).collect()
}

fn sorted_tag_names(tags: HashSet<String>) -> Vec<String> {
    let mut tags: Vec<_> = tags.into_iter().collect();
    tags.sort_unstable();
    tags
}

fn sorted_object_ids(ids: HashSet<iota_interaction::types::base_types::ObjectID>) -> Vec<String> {
    let mut ids: Vec<_> = ids.into_iter().map(|id| id.to_string()).collect();
    ids.sort_unstable();
    ids
}

fn optional_object_id(id: Option<ObjectID>) -> Option<String> {
    id.map(|id| id.to_string())
}

fn sorted_role_entries(roles: HashMap<String, Role>) -> Vec<WasmRolePermissionsEntry> {
    let mut roles: Vec<_> = roles
        .into_iter()
        .map(|(name, role)| WasmRolePermissionsEntry {
            name,
            permissions: sorted_permissions_from_set(role.permissions),
            role_tags: role.data.map(Into::into),
        })
        .collect();
    roles.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    roles
}

/// Permission variants enumerated by the audit trail.
///
/// @remarks
/// Each variant authorizes one operation on a trail. Variants are grouped by the proposed role
/// that typically owns them (`Admin`, `RecordAdmin`, `LockingAdmin`, `RoleAdmin`, `CapAdmin`,
/// `MetadataAdmin`, `TagAdmin`); see {@link PermissionSet} for the recommended sets.
#[wasm_bindgen(js_name = Permission)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmPermission {
    /// Authorizes deleting the trail itself.
    DeleteAuditTrail,
    /// Authorizes the batched record-deletion entry point.
    DeleteAllRecords,
    /// Authorizes appending a record.
    AddRecord,
    /// Authorizes deleting an individual record.
    DeleteRecord,
    /// Authorizes adding a record that supersedes earlier records via `RecordCorrection`.
    CorrectRecord,
    /// Authorizes replacing the full {@link LockingConfig}.
    UpdateLockingConfig,
    /// Authorizes updating only the delete-record window of the locking configuration.
    UpdateLockingConfigForDeleteRecord,
    /// Authorizes updating only the delete-trail lock of the locking configuration.
    UpdateLockingConfigForDeleteTrail,
    /// Authorizes updating only the write lock of the locking configuration.
    UpdateLockingConfigForWrite,
    /// Authorizes creating roles.
    AddRoles,
    /// Authorizes updating existing roles.
    UpdateRoles,
    /// Authorizes deleting roles.
    DeleteRoles,
    /// Authorizes issuing capabilities.
    AddCapabilities,
    /// Authorizes revoking, destroying, and cleaning up capabilities.
    RevokeCapabilities,
    /// Authorizes replacing the trail's `updatableMetadata`.
    UpdateMetadata,
    /// Authorizes clearing the trail's `updatableMetadata`.
    DeleteMetadata,
    /// Authorizes the migration entry point used after package upgrades.
    Migrate,
    /// Authorizes adding entries to the trail's record-tag registry.
    AddRecordTags,
    /// Authorizes removing entries from the trail's record-tag registry.
    DeleteRecordTags,
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
            Permission::AddRecordTags => Self::AddRecordTags,
            Permission::DeleteRecordTags => Self::DeleteRecordTags,
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
            WasmPermission::AddRecordTags => Self::AddRecordTags,
            WasmPermission::DeleteRecordTags => Self::DeleteRecordTags,
        }
    }
}

/// Set of permissions granted by a role.
#[wasm_bindgen(js_name = PermissionSet, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmPermissionSet {
    /// Permissions granted by this set.
    pub permissions: Vec<WasmPermission>,
}

#[wasm_bindgen(js_class = PermissionSet)]
impl WasmPermissionSet {
    /// Creates a permission set from an explicit list of permissions.
    ///
    /// @param permissions - Permissions to include in the set.
    #[wasm_bindgen(constructor)]
    pub fn new(permissions: Vec<WasmPermission>) -> Self {
        Self { permissions }
    }

    /// Returns the recommended permission set for the reserved initial-admin role.
    ///
    /// @returns A {@link PermissionSet} that authorizes role and capability administration.
    #[wasm_bindgen(js_name = adminPermissions)]
    pub fn admin_permissions() -> Self {
        PermissionSet::admin_permissions().into()
    }

    /// Returns the permissions needed to administer records.
    ///
    /// @returns A {@link PermissionSet} that authorizes record reads, writes, and deletions.
    #[wasm_bindgen(js_name = recordAdminPermissions)]
    pub fn record_admin_permissions() -> Self {
        PermissionSet::record_admin_permissions().into()
    }

    /// Returns the permissions needed to administer locking rules.
    ///
    /// @returns A {@link PermissionSet} that authorizes updates to all locking dimensions.
    #[wasm_bindgen(js_name = lockingAdminPermissions)]
    pub fn locking_admin_permissions() -> Self {
        PermissionSet::locking_admin_permissions().into()
    }

    /// Returns the permissions needed to administer roles.
    ///
    /// @returns A {@link PermissionSet} that authorizes adding, updating, and deleting roles.
    #[wasm_bindgen(js_name = roleAdminPermissions)]
    pub fn role_admin_permissions() -> Self {
        PermissionSet::role_admin_permissions().into()
    }

    /// Returns the permissions needed to issue and revoke capabilities.
    ///
    /// @returns A {@link PermissionSet} that authorizes the capability lifecycle.
    #[wasm_bindgen(js_name = capAdminPermissions)]
    pub fn cap_admin_permissions() -> Self {
        PermissionSet::cap_admin_permissions().into()
    }

    /// Returns the permissions needed to administer mutable metadata.
    ///
    /// @returns A {@link PermissionSet} that authorizes updating and clearing
    /// `updatableMetadata`.
    #[wasm_bindgen(js_name = metadataAdminPermissions)]
    pub fn metadata_admin_permissions() -> Self {
        PermissionSet::metadata_admin_permissions().into()
    }

    /// Returns the permissions needed to administer record tags.
    ///
    /// @returns A {@link PermissionSet} that authorizes adding and removing entries from the
    /// trail's record-tag registry.
    #[wasm_bindgen(js_name = tagAdminPermissions)]
    pub fn tag_admin_permissions() -> Self {
        PermissionSet::tag_admin_permissions().into()
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

/// Linked-table metadata for record storage.
#[wasm_bindgen(js_name = LinkedTable, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmLinkedTable {
    /// Linked-table object ID.
    pub id: String,
    /// Declared number of entries in the table.
    pub size: u64,
    /// Sequence number of the first entry, if any.
    pub head: Option<u64>,
    /// Sequence number of the last entry, if any.
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

/// Permissions required to administer roles, as enforced by the trail.
#[wasm_bindgen(js_name = RoleAdminPermissions, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmRoleAdminPermissions {
    /// Permission required to create roles.
    pub add: WasmPermission,
    /// Permission required to delete roles.
    pub delete: WasmPermission,
    /// Permission required to update roles.
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

/// Permissions required to administer capabilities, as enforced by the trail.
#[wasm_bindgen(js_name = CapabilityAdminPermissions, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmCapabilityAdminPermissions {
    /// Permission required to issue capabilities.
    pub add: WasmPermission,
    /// Permission required to revoke capabilities.
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

/// Flattened role entry exposed inside {@link RoleMap}.
#[wasm_bindgen(js_name = RolePermissionsEntry, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRolePermissionsEntry {
    /// Role name.
    pub name: String,
    /// Permissions granted by the role.
    pub permissions: Vec<WasmPermission>,
    /// Optional role-scoped record-tag restrictions.
    #[wasm_bindgen(js_name = roleTags)]
    pub role_tags: Option<WasmRoleTags>,
}

/// Allowlisted record tags stored on a role.
///
/// @remarks
/// Every tag listed here must already exist in the trail's record-tag registry before the role is
/// created or updated; otherwise the on-chain call aborts.
#[wasm_bindgen(js_name = RoleTags, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmRoleTags {
    /// Sorted tag names allowed by the role.
    pub tags: Vec<String>,
}

#[wasm_bindgen(js_class = RoleTags)]
impl WasmRoleTags {
    /// Creates role-tag restrictions from a list of tag names.
    ///
    /// @remarks
    /// The supplied names are sorted alphabetically and de-duplicated.
    ///
    /// @param tags - Tag names allowed by the role.
    #[wasm_bindgen(constructor)]
    pub fn new(tags: Vec<String>) -> Self {
        let mut tags = tags;
        tags.sort_unstable();
        tags.dedup();
        Self { tags }
    }
}

impl From<RoleTags> for WasmRoleTags {
    fn from(value: RoleTags) -> Self {
        Self {
            tags: sorted_tag_names(value.tags),
        }
    }
}

impl From<WasmRoleTags> for RoleTags {
    fn from(value: WasmRoleTags) -> Self {
        RoleTags::new(value.tags)
    }
}

/// Trail-owned record tag plus its usage count.
#[wasm_bindgen(js_name = RecordTagEntry, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmRecordTagEntry {
    /// Tag name.
    pub tag: String,
    /// Combined number of records and roles currently referencing the tag.
    #[wasm_bindgen(js_name = usageCount)]
    pub usage_count: u64,
}

impl From<(String, u64)> for WasmRecordTagEntry {
    fn from((tag, usage_count): (String, u64)) -> Self {
        Self { tag, usage_count }
    }
}

/// Snapshot of the trail's role map.
///
/// @remarks
/// Mirrors the access-control state maintained by the audit-trail package, including the reserved
/// initial-admin role, the revoked-capability denylist, and the role data used for tag-aware
/// authorization.
#[wasm_bindgen(js_name = RoleMap, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRoleMap {
    /// Trail object ID that this role map protects.
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    /// Role definitions sorted by role name.
    pub roles: Vec<WasmRolePermissionsEntry>,
    /// Reserved role name used for initial-admin capabilities.
    ///
    /// Always equals `"Admin"`. The role bearing this name cannot be deleted.
    #[wasm_bindgen(js_name = initialAdminRoleName)]
    pub initial_admin_role_name: String,
    /// Denylist of revoked capability IDs.
    #[wasm_bindgen(js_name = revokedCapabilities)]
    pub revoked_capabilities: WasmObjectIdLinkedTable,
    /// Capability IDs currently recognized as initial-admin capabilities.
    #[wasm_bindgen(js_name = initialAdminCapIds)]
    pub initial_admin_cap_ids: Vec<String>,
    /// Permissions required to administer roles.
    #[wasm_bindgen(js_name = roleAdminPermissions)]
    pub role_admin_permissions: WasmRoleAdminPermissions,
    /// Permissions required to administer capabilities.
    #[wasm_bindgen(js_name = capabilityAdminPermissions)]
    pub capability_admin_permissions: WasmCapabilityAdminPermissions,
}

impl From<RoleMap> for WasmRoleMap {
    fn from(value: RoleMap) -> Self {
        Self {
            target_key: value.target_key.to_string(),
            roles: sorted_role_entries(value.roles),
            initial_admin_role_name: value.initial_admin_role_name,
            revoked_capabilities: value.revoked_capabilities.into(),
            initial_admin_cap_ids: sorted_object_ids(value.initial_admin_cap_ids),
            role_admin_permissions: value.role_admin_permissions.into(),
            capability_admin_permissions: value.capability_admin_permissions.into(),
        }
    }
}

/// Linked-table metadata keyed by object IDs.
#[wasm_bindgen(js_name = ObjectIdLinkedTable, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmObjectIdLinkedTable {
    /// Linked-table object ID.
    pub id: String,
    /// Declared number of entries in the table.
    pub size: u64,
    /// Object ID of the first entry, if any.
    pub head: Option<String>,
    /// Object ID of the last entry, if any.
    pub tail: Option<String>,
}

impl From<LinkedTable<ObjectID>> for WasmObjectIdLinkedTable {
    fn from(value: LinkedTable<ObjectID>) -> Self {
        Self {
            id: value.id.to_string(),
            size: value.size,
            head: optional_object_id(value.head),
            tail: optional_object_id(value.tail),
        }
    }
}

/// Capability issuance options.
///
/// @remarks
/// These fields configure restrictions on the issued capability object. Matching against the
/// current caller and the on-chain timestamp happens whenever the capability is later presented
/// for authorization, not at issue time.
#[wasm_bindgen(js_name = CapabilityIssueOptions, getter_with_clone, inspectable)]
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct WasmCapabilityIssueOptions {
    /// Address that should own the issued capability. When `null`, the capability is transferred
    /// to the caller.
    #[wasm_bindgen(js_name = issuedTo)]
    pub issued_to: Option<WasmIotaAddress>,
    /// Earliest millisecond timestamp (since the Unix epoch) at which the capability becomes
    /// valid. When `null`, the capability is valid from its creation time.
    #[wasm_bindgen(js_name = validFromMs)]
    pub valid_from_ms: Option<u64>,
    /// Latest millisecond timestamp (since the Unix epoch) at which the capability is still
    /// valid. When `null`, the capability does not expire.
    #[wasm_bindgen(js_name = validUntilMs)]
    pub valid_until_ms: Option<u64>,
}

#[wasm_bindgen(js_class = CapabilityIssueOptions)]
impl WasmCapabilityIssueOptions {
    /// Creates capability issuance options.
    ///
    /// @param issuedTo - Optional recipient address; `null` keeps the capability with the caller.
    /// @param validFromMs - Optional earliest valid timestamp in milliseconds since the Unix
    /// epoch.
    /// @param validUntilMs - Optional latest valid timestamp in milliseconds since the Unix epoch.
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

/// Capability data describing a granted role and its validity window.
///
/// @remarks
/// A capability grants exactly one role against exactly one trail and may additionally restrict
/// who may use it and during which time window it is valid.
#[wasm_bindgen(js_name = Capability, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmCapability {
    /// Capability object ID.
    pub id: String,
    /// Trail object ID protected by the capability.
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    /// Role granted by the capability.
    pub role: String,
    /// Address bound to the capability. When `null`, any holder may present the capability for
    /// authorization.
    #[wasm_bindgen(js_name = issuedTo)]
    pub issued_to: Option<WasmIotaAddress>,
    /// Earliest millisecond timestamp (since the Unix epoch, inclusive) at which the capability
    /// is valid. When `null`, the capability is valid from its creation time.
    #[wasm_bindgen(js_name = validFrom)]
    pub valid_from: Option<u64>,
    /// Latest millisecond timestamp (since the Unix epoch, inclusive) at which the capability is
    /// still valid. When `null`, the capability does not expire.
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

/// Event payload emitted when a trail is created.
#[wasm_bindgen(js_name = AuditTrailCreated, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmAuditTrailCreated {
    /// Newly created trail object ID.
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    /// Address that created the trail.
    pub creator: WasmIotaAddress,
    /// Millisecond event timestamp.
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

/// Event payload emitted when a trail is deleted.
#[wasm_bindgen(js_name = AuditTrailDeleted, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmAuditTrailDeleted {
    /// Deleted trail object ID.
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    /// Millisecond event timestamp.
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

/// Event payload emitted when a record is added.
#[wasm_bindgen(js_name = RecordAdded, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRecordAdded {
    /// Trail object ID receiving the new record.
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    /// Sequence number assigned to the new record.
    #[wasm_bindgen(js_name = sequenceNumber)]
    pub sequence_number: u64,
    /// Address that added the record.
    #[wasm_bindgen(js_name = addedBy)]
    pub added_by: WasmIotaAddress,
    /// Millisecond event timestamp.
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

/// Event payload emitted when a record is deleted.
#[wasm_bindgen(js_name = RecordDeleted, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRecordDeleted {
    /// Trail object ID from which the record was deleted.
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    /// Sequence number of the deleted record.
    #[wasm_bindgen(js_name = sequenceNumber)]
    pub sequence_number: u64,
    /// Address that deleted the record.
    #[wasm_bindgen(js_name = deletedBy)]
    pub deleted_by: WasmIotaAddress,
    /// Millisecond event timestamp.
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

/// Event payload emitted when a capability is issued.
#[wasm_bindgen(js_name = CapabilityIssued, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmCapabilityIssued {
    /// Trail object ID protected by the capability.
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    /// Newly created capability object ID.
    #[wasm_bindgen(js_name = capabilityId)]
    pub capability_id: String,
    /// Role granted by the capability.
    pub role: String,
    /// Address bound to the capability, if one was assigned at issue time.
    #[wasm_bindgen(js_name = issuedTo)]
    pub issued_to: Option<WasmIotaAddress>,
    /// Earliest millisecond timestamp (since the Unix epoch, inclusive) at which the capability
    /// becomes valid. `null` when no lower bound was set.
    #[wasm_bindgen(js_name = validFrom)]
    pub valid_from: Option<u64>,
    /// Latest millisecond timestamp (since the Unix epoch, inclusive) at which the capability is
    /// still valid. `null` when no expiry was set.
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

/// Event payload emitted when a capability is destroyed.
#[wasm_bindgen(js_name = CapabilityDestroyed, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmCapabilityDestroyed {
    /// Trail object ID protected by the capability.
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    /// Destroyed capability object ID.
    #[wasm_bindgen(js_name = capabilityId)]
    pub capability_id: String,
    /// Role granted by the capability.
    pub role: String,
    /// Address bound to the capability, if one had been assigned.
    #[wasm_bindgen(js_name = issuedTo)]
    pub issued_to: Option<WasmIotaAddress>,
    /// Earliest millisecond timestamp (since the Unix epoch, inclusive) at which the capability
    /// became valid. `null` when no lower bound had been set.
    #[wasm_bindgen(js_name = validFrom)]
    pub valid_from: Option<u64>,
    /// Latest millisecond timestamp (since the Unix epoch, inclusive) at which the capability had
    /// been valid. `null` when no expiry had been set.
    #[wasm_bindgen(js_name = validUntil)]
    pub valid_until: Option<u64>,
}

impl From<CapabilityDestroyed> for WasmCapabilityDestroyed {
    fn from(value: CapabilityDestroyed) -> Self {
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

/// Event payload emitted when a capability is revoked.
#[wasm_bindgen(js_name = CapabilityRevoked, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmCapabilityRevoked {
    /// Trail object ID protected by the capability.
    #[wasm_bindgen(js_name = targetKey)]
    pub target_key: String,
    /// Revoked capability object ID.
    #[wasm_bindgen(js_name = capabilityId)]
    pub capability_id: String,
    /// Millisecond timestamp retained for denylist cleanup.
    ///
    /// `0` when the capability had no expiry — denylist entries with `validUntil == 0` are kept
    /// indefinitely.
    #[wasm_bindgen(js_name = validUntil)]
    pub valid_until: u64,
}

impl From<CapabilityRevoked> for WasmCapabilityRevoked {
    fn from(value: CapabilityRevoked) -> Self {
        Self {
            target_key: value.target_key.to_string(),
            capability_id: value.capability_id.to_string(),
            valid_until: value.valid_until,
        }
    }
}

/// Event payload emitted when expired revoked-capability entries are cleaned up.
#[wasm_bindgen(js_name = RevokedCapabilitiesCleanedUp, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRevokedCapabilitiesCleanedUp {
    /// Trail object ID whose denylist was pruned.
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    /// Number of expired entries removed by this cleanup call.
    #[wasm_bindgen(js_name = cleanedCount)]
    pub cleaned_count: u64,
    /// Address that triggered the cleanup.
    #[wasm_bindgen(js_name = cleanedBy)]
    pub cleaned_by: WasmIotaAddress,
    /// Millisecond event timestamp.
    pub timestamp: u64,
}

impl From<RevokedCapabilitiesCleanedUp> for WasmRevokedCapabilitiesCleanedUp {
    fn from(value: RevokedCapabilitiesCleanedUp) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            cleaned_count: value.cleaned_count,
            cleaned_by: value.cleaned_by.to_string(),
            timestamp: value.timestamp,
        }
    }
}

/// Event payload emitted when a role is created.
#[wasm_bindgen(js_name = RoleCreated, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRoleCreated {
    /// Trail object ID that owns the role.
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    /// Role name.
    pub role: String,
    /// Permissions granted by the new role.
    pub permissions: WasmPermissionSet,
    /// Optional record-tag restrictions stored as role data.
    #[wasm_bindgen(js_name = roleTags)]
    pub role_tags: Option<WasmRoleTags>,
    /// Address that created the role.
    #[wasm_bindgen(js_name = createdBy)]
    pub created_by: WasmIotaAddress,
    /// Millisecond event timestamp.
    pub timestamp: u64,
}

impl From<RoleCreated> for WasmRoleCreated {
    fn from(value: RoleCreated) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            role: value.role,
            permissions: value.permissions.into(),
            role_tags: value.data.map(Into::into),
            created_by: value.created_by.to_string(),
            timestamp: value.timestamp,
        }
    }
}

/// Event payload emitted when a role is updated.
#[wasm_bindgen(js_name = RoleUpdated, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRoleUpdated {
    /// Trail object ID that owns the role.
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    /// Role name.
    pub role: String,
    /// Updated permissions for the role.
    pub permissions: WasmPermissionSet,
    /// Updated record-tag restrictions, if any.
    #[wasm_bindgen(js_name = roleTags)]
    pub role_tags: Option<WasmRoleTags>,
    /// Address that updated the role.
    #[wasm_bindgen(js_name = updatedBy)]
    pub updated_by: WasmIotaAddress,
    /// Millisecond event timestamp.
    pub timestamp: u64,
}

impl From<RoleUpdated> for WasmRoleUpdated {
    fn from(value: RoleUpdated) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            role: value.role,
            permissions: value.permissions.into(),
            role_tags: value.data.map(Into::into),
            updated_by: value.updated_by.to_string(),
            timestamp: value.timestamp,
        }
    }
}

/// Event payload emitted when a role is deleted.
#[wasm_bindgen(js_name = RoleDeleted, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRoleDeleted {
    /// Trail object ID that owned the role.
    #[wasm_bindgen(js_name = trailId)]
    pub trail_id: String,
    /// Role name.
    pub role: String,
    /// Address that deleted the role.
    #[wasm_bindgen(js_name = deletedBy)]
    pub deleted_by: WasmIotaAddress,
    /// Millisecond event timestamp.
    pub timestamp: u64,
}

impl From<RoleDeleted> for WasmRoleDeleted {
    fn from(value: RoleDeleted) -> Self {
        Self {
            trail_id: value.trail_id.to_string(),
            role: value.role,
            deleted_by: value.deleted_by.to_string(),
            timestamp: value.timestamp,
        }
    }
}

/// Discriminant for the shape stored inside {@link TimeLock}.
#[wasm_bindgen(js_name = TimeLockType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmTimeLockType {
    /// The time lock is disabled.
    None,
    /// The lock unlocks at a Unix timestamp in seconds.
    UnlockAt,
    /// The lock unlocks at a Unix timestamp in milliseconds.
    UnlockAtMs,
    /// The lock stays active until the protected object is explicitly destroyed.
    ///
    /// Not supported as the trail-delete lock.
    UntilDestroyed,
    /// The lock is always active.
    Infinite,
}

/// Time-based lock used in the trail's {@link LockingConfig}.
///
/// @remarks
/// {@link TimeLock.withUntilDestroyed} is rejected by the audit-trail package when used as the
/// trail-delete lock; pass it only for the write lock.
#[wasm_bindgen(js_name = TimeLock, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmTimeLock(pub(crate) TimeLock);

#[wasm_bindgen(js_class = TimeLock)]
impl WasmTimeLock {
    /// Creates a lock that unlocks at a Unix timestamp in seconds.
    ///
    /// @param timeSec - Unlock time in seconds since the Unix epoch.
    ///
    /// @returns A lock that unlocks once the on-chain clock reaches `timeSec`.
    #[wasm_bindgen(js_name = withUnlockAt)]
    pub fn with_unlock_at(time_sec: u32) -> Self {
        Self(TimeLock::UnlockAt(time_sec))
    }

    /// Creates a lock that unlocks at a Unix timestamp in milliseconds.
    ///
    /// @param timeMs - Unlock time in milliseconds since the Unix epoch.
    ///
    /// @returns A lock that unlocks once the on-chain clock reaches `timeMs`.
    #[wasm_bindgen(js_name = withUnlockAtMs)]
    pub fn with_unlock_at_ms(time_ms: u64) -> Self {
        Self(TimeLock::UnlockAtMs(time_ms))
    }

    /// Creates a lock that stays active until the protected object is destroyed.
    ///
    /// @returns A lock that remains active until the protected object is destroyed.
    #[wasm_bindgen(js_name = withUntilDestroyed)]
    pub fn with_until_destroyed() -> Self {
        Self(TimeLock::UntilDestroyed)
    }

    /// Creates a lock that never unlocks.
    ///
    /// @returns A lock that is always active.
    #[wasm_bindgen(js_name = withInfinite)]
    pub fn with_infinite() -> Self {
        Self(TimeLock::Infinite)
    }

    /// Creates a disabled lock.
    ///
    /// @returns A lock that does not gate the protected operation.
    #[wasm_bindgen(js_name = withNone)]
    pub fn with_none() -> Self {
        Self(TimeLock::None)
    }

    /// Returns the lock variant.
    ///
    /// @returns The {@link TimeLockType} discriminant for this lock.
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

    /// Returns the lock argument for parameterized variants.
    ///
    /// @returns The numeric argument for `UnlockAt`/`UnlockAtMs` variants, or `undefined`
    /// otherwise.
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

/// Discriminant for the shape stored inside {@link LockingWindow}.
#[wasm_bindgen(js_name = LockingWindowType)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WasmLockingWindowType {
    /// No delete window is enforced; records may be deleted at any time.
    None,
    /// The window locks records while their age is below a configured number of seconds.
    TimeBased,
    /// The window locks records while they are among the most recent N records.
    CountBased,
}

/// Delete-window definition used in the trail's {@link LockingConfig}.
///
/// @remarks
/// A window describes the period during which a record stays *locked against deletion*: time-based
/// windows lock a record while its age is below the configured number of seconds; count-based
/// windows lock a record while it is among the most recent N records. Records outside the window
/// may be deleted, subject to remaining permission and tag checks.
#[wasm_bindgen(js_name = LockingWindow, inspectable)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmLockingWindow(pub(crate) LockingWindow);

#[wasm_bindgen(js_class = LockingWindow)]
impl WasmLockingWindow {
    /// Creates a disabled delete window.
    ///
    /// @returns A window that does not lock records against deletion.
    #[wasm_bindgen(js_name = withNone)]
    pub fn with_none() -> Self {
        Self(LockingWindow::None)
    }

    /// Creates a time-based delete window.
    ///
    /// @param seconds - Maximum record age, in seconds, for which the record stays locked against
    /// deletion.
    ///
    /// @returns A window that locks records younger than `seconds`.
    #[wasm_bindgen(js_name = withTimeBased)]
    pub fn with_time_based(seconds: u64) -> Self {
        Self(LockingWindow::TimeBased { seconds })
    }

    /// Creates a count-based delete window.
    ///
    /// @param count - Number of most recent records that stay locked against deletion.
    ///
    /// @returns A window that locks the `count` most recent records.
    #[wasm_bindgen(js_name = withCountBased)]
    pub fn with_count_based(count: u64) -> Self {
        Self(LockingWindow::CountBased { count })
    }

    /// Returns the window variant.
    ///
    /// @returns The {@link LockingWindowType} discriminant for this window.
    #[wasm_bindgen(js_name = "type", getter)]
    pub fn window_type(&self) -> WasmLockingWindowType {
        match self.0 {
            LockingWindow::None => WasmLockingWindowType::None,
            LockingWindow::TimeBased { .. } => WasmLockingWindowType::TimeBased,
            LockingWindow::CountBased { .. } => WasmLockingWindowType::CountBased,
        }
    }

    /// Returns the window argument for parameterized variants.
    ///
    /// @returns The numeric argument for `TimeBased`/`CountBased` variants, or `undefined`
    /// otherwise.
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

/// Full locking configuration.
///
/// @remarks
/// Combines three independent rules: a per-record delete window, a trail-delete time lock, and a
/// write-time lock. The trail-delete lock must not be {@link TimeLock.withUntilDestroyed}; trail
/// creation and locking updates that violate this invariant abort on-chain.
#[wasm_bindgen(js_name = LockingConfig, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmLockingConfig {
    /// Delete-window policy applied to individual records.
    ///
    /// Records inside the window are locked against deletion.
    #[wasm_bindgen(js_name = deleteRecordWindow)]
    pub delete_record_window: WasmLockingWindow,
    /// Time lock that gates deletion of the entire trail.
    ///
    /// Must not be {@link TimeLock.withUntilDestroyed}; trail creation and locking updates that
    /// violate this invariant abort on-chain.
    #[wasm_bindgen(js_name = deleteTrailLock)]
    pub delete_trail_lock: WasmTimeLock,
    /// Time lock that gates record writes (`addRecord`).
    #[wasm_bindgen(js_name = writeLock)]
    pub write_lock: WasmTimeLock,
}

#[wasm_bindgen(js_class = LockingConfig)]
impl WasmLockingConfig {
    /// Creates a locking configuration.
    ///
    /// @param deleteRecordWindow - {@link LockingWindow} that controls when individual records may
    /// be deleted.
    /// @param deleteTrailLock - {@link TimeLock} that controls when the trail itself may be
    /// deleted.
    /// @param writeLock - {@link TimeLock} that controls when records may be appended.
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

/// Immutable trail metadata.
///
/// @remarks
/// Stored once on the trail object at creation and exposed read-only thereafter. Use
/// {@link OnChainAuditTrail.updatableMetadata} for the mutable counterpart.
#[wasm_bindgen(js_name = ImmutableMetadata, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmImmutableMetadata {
    /// Human-readable trail name.
    pub name: String,
    /// Optional human-readable description.
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

/// Correction metadata attached to a record.
///
/// @remarks
/// {@link RecordCorrection.replaces} is fixed at record creation and lists the sequence numbers
/// this record supersedes; {@link RecordCorrection.isReplacedBy} is a back-pointer the trail sets
/// later when this record itself is corrected.
#[wasm_bindgen(js_name = RecordCorrection, getter_with_clone, inspectable)]
#[derive(Clone, Serialize, Deserialize)]
pub struct WasmRecordCorrection {
    /// Sorted sequence numbers that this record supersedes.
    pub replaces: Vec<u64>,
    /// Sequence number of the record that supersedes this one, if any.
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

/// Single audit-trail record.
///
/// @remarks
/// Records form a tamper-evident, sequential chain: each record has a monotonically increasing
/// sequence number that is never reused, even after the record is deleted.
#[wasm_bindgen(js_name = Record, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmRecord {
    /// Record payload stored on-chain.
    pub data: WasmData,
    /// Optional application-defined metadata.
    pub metadata: Option<String>,
    /// Optional trail-owned tag attached to the record.
    pub tag: Option<String>,
    /// Monotonic record sequence number inside the trail.
    #[wasm_bindgen(js_name = sequenceNumber)]
    pub sequence_number: u64,
    /// Address that added the record.
    #[wasm_bindgen(js_name = addedBy)]
    pub added_by: WasmIotaAddress,
    /// Millisecond timestamp at which the record was added.
    #[wasm_bindgen(js_name = addedAt)]
    pub added_at: u64,
    /// Correction relationships for this record.
    pub correction: WasmRecordCorrection,
}

impl From<Record<Data>> for WasmRecord {
    fn from(value: Record<Data>) -> Self {
        Self {
            data: value.data.into(),
            metadata: value.metadata,
            tag: value.tag,
            sequence_number: value.sequence_number,
            added_by: value.added_by.to_string(),
            added_at: value.added_at,
            correction: value.correction.into(),
        }
    }
}

/// One page of records returned by {@link TrailRecords.listPage}.
#[wasm_bindgen(js_name = PaginatedRecord, getter_with_clone, inspectable)]
#[derive(Clone)]
pub struct WasmPaginatedRecord {
    /// Records included in the current page, ordered by sequence number.
    pub records: Vec<WasmRecord>,
    /// Cursor to pass to the next {@link TrailRecords.listPage} call.
    #[wasm_bindgen(js_name = nextCursor)]
    pub next_cursor: Option<u64>,
    /// Indicates whether another page may be available.
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
