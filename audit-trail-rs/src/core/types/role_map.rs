// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::collection_types::LinkedTable;
use iota_interaction::types::id::UID;
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use iota_interaction::{MoveType, ident_str};
use serde::{Deserialize, Serialize};
use serde_aux::field_attributes::deserialize_option_number_from_string;

use super::permission::Permission;
use crate::core::internal::move_collections::{deserialize_vec_map, deserialize_vec_set};
use crate::core::internal::tx;
use crate::error::Error;

/// The role and capability registry attached to an audit trail.
///
/// A [`RoleMap`] stores every named role defined on the trail, tracks which
/// capabilities have been revoked, and records the administrative permission
/// requirements for role and capability management.
///
/// ## Roles and capabilities
///
/// Each entry in [`roles`](RoleMap::roles) maps a role name to a [`Role`] that
/// holds a set of [`Permission`]s and [`RoleTags`].
/// Each [`Capability`] is associated with exactly one  [`Role`] and belongs to a specific [`AuditTrail`]
/// instance which is identified by the [`target_key`](RoleMap::target_key).
///
/// ## What are Roles
///
/// A role is a named set of [`Permission`]s, optionally paired with a [`RoleTags`] allowlist.
///
/// Roles are identified by a unique string name within a trail (e.g., `"RecordAdmin"`,
/// `"Auditor"`, `"LegalReviewer"`). The same role definition can back many independent
/// [`Capability`] objects — to be owned and used by users or system components that should share the same
/// access level.  A capability holder may exercise only the permissions of the role it was
/// issued for.
///
/// ## Initial admin role and capability
///
/// When a trail is created the Move runtime mints an *initial admin*
/// capability and transfers it to the creator (or the address supplied via
/// `with_admin`).
///
/// The *initial admin* role name is indicated by [`initial_admin_role_name`].
/// The role grants permissions specified by [`role_admin_permissions`] and [`capability_admin_permissions`],
/// which are required to manage additional roles and capabilities.
///
/// ## Create, Delete and Update Roles
///
/// All three operations are gated by the permissions stored in
/// [`role_admin_permissions`](RoleMap::role_admin_permissions):
///
/// | Operation | Required permission                  | Additional constraints                                                                                          |
/// |-----------|--------------------------------------|-----------------------------------------------------------------------------------------------------------------|
/// | Create    | `role_admin_permissions.add`         | Any [`RoleTags`] specified must be registered in the trail's tag registry.                                      |
/// | Delete    | `role_admin_permissions.delete`      | The initial admin role (see [`initial_admin_role_name`](RoleMap::initial_admin_role_name)) cannot be deleted.   |
/// | Update    | `role_admin_permissions.update`      | Updating the initial admin role requires the new permission set to still include all configured admin permissions.|
///
/// The caller supplies a [`Capability`] that is validated before the operation proceeds.
/// An `ECapabilityPermissionDenied` error is returned if the capability's role does not
/// carry the required permission.
///
/// ## Issue, Revoke, and Destroy Capabilities
///
/// **Issuing** a capability requires the `capability_admin_permissions.add` permission.
/// [`CapabilityIssueOptions`] allow restricting a newly minted capability further:
/// - `issued_to` — binds the capability to a specific wallet address; the Move runtime rejects use by any other sender.
/// - `valid_from_ms` / `valid_until_ms` — a Unix-millisecond validity window; use outside this range is rejected.
///
/// **Revoking** a capability requires the `capability_admin_permissions.revoke` permission.
/// Revocation adds the capability's ID to the [`revoked_capabilities`](RoleMap::revoked_capabilities)
/// denylist; the object itself continues to exist on-chain but is refused by
/// `assert_capability_valid`.  The caller must provide:
/// - the capability's object ID, and
/// - optionally its `valid_until` value, which allows the denylist entry to be cleaned up automatically once it expires
///   via [`AuditTrailHandle::access().cleanup_revoked_capabilities`].
///
/// Because the `RoleMap` uses a denylist (not an allowlist), it does **not** track all
/// issued capabilities on-chain.  Callers are responsible for maintaining an off-chain
/// record of issued capability IDs and their validity constraints so that the correct ID
/// can be supplied at revocation time.
///
/// **Destroying** a capability permanently removes it from the chain.  Any holder may
/// destroy their own capability without needing any admin permission — this is intentional
/// so that users can always clean up capabilities they no longer need.  Destroying a
/// revoked capability also removes it from the denylist.
///
/// ## Managing the initial admin role and its capabilities
///
/// The initial admin role is the only role that exists when a trail is first created.
/// It carries all permissions required to manage roles and capabilities
/// (i.e. everything in [`role_admin_permissions`](RoleMap::role_admin_permissions) and
/// [`capability_admin_permissions`](RoleMap::capability_admin_permissions)).
///
/// Two invariants protect it from accidental lock-out:
/// - The initial admin **role** can never be deleted.
/// - Updating its permissions is only permitted if the new permission set still includes all configured role and
///   capability admin permissions.
///
/// Initial admin **capabilities** are tracked separately in
/// [`initial_admin_cap_ids`](RoleMap::initial_admin_cap_ids) and must be managed through
/// dedicated entry-points:
/// - `revoke_initial_admin_capability` — adds the cap to the denylist.
/// - `destroy_initial_admin_capability` — permanently removes the cap from the chain.
///
/// Attempting to use the generic `revoke_capability` or `destroy_capability` on an initial
/// admin capability returns `EInitialAdminCapabilityMustBeExplicitlyDestroyed`.
///
/// ## Using Tags
/// Tags are string labels managed by the audit trail using a [`TagRegistry`](super::audit_trail::TagRegistry).
/// The registry acts as a controlled vocabulary: a tag must be registered on the
/// trail before it can be attached to a record or referenced by a role.
///
/// Each record may carry at most one immutable tag. Tagged Records can only be accessed
/// by users having Capabilities with Roles that allow the tag in their RoleTags.
/// This allows for flexible access control policies based on record tags,
/// i.e. to allow access to specific records only for users in specific departments.
///
/// Each role may optionally include a [`RoleTags`] allowlist that grants the holders of that
/// role's capability access to records tagged with that specific tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMap {
    /// The object ID of the audit trail this role map belongs to.
    pub target_key: ObjectID,
    /// All named roles defined on the trail, keyed by role name.
    #[serde(deserialize_with = "deserialize_vec_map")]
    pub roles: HashMap<String, Role>,
    /// Name of the built-in admin role created automatically at trail creation
    /// (typically `"Admin"`).
    pub initial_admin_role_name: String,
    /// Set of capability IDs that have been revoked and must no longer be
    /// accepted by the Move runtime.
    pub revoked_capabilities: LinkedTable<ObjectID>,
    /// Object IDs of the initial admin capabilities minted at trail creation.
    /// These require dedicated revoke/destroy entry-points.
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub initial_admin_cap_ids: HashSet<ObjectID>,
    /// Permissions required to add, update, and delete roles on this trail.
    pub role_admin_permissions: RoleAdminPermissions,
    /// Permissions required to issue and revoke capabilities on this trail.
    pub capability_admin_permissions: CapabilityAdminPermissions,
}

/// A single role definition within a [`RoleMap`].
///
/// Each role combines a permission set that governs what operations holders may
/// perform, and optional [`RoleTags`] data that restricts which tagged records
/// those holders may interact with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// The set of [`Permission`]s granted to any [`Capability`] issued for this role.
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub permissions: HashSet<Permission>,
    /// Optional tag allowlist.  When present, a capability holder for this role may only
    /// add or access records whose tag is contained in this set.  When `None`, the role
    /// does not impose any tag-based restriction (but untagged-record permissions still
    /// apply).
    pub data: Option<RoleTags>,
}

/// Defines the permissions required to administer roles in this [`RoleMap`].
///
/// When a capability holder attempts to create, delete, or update a role, the
/// `RoleMap` checks that the holder's role includes the corresponding permission
/// listed here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAdminPermissions {
    /// The [`Permission`] a capability must carry to create a new role
    /// (typically [`Permission::AddRoles`]).
    pub add: Permission,
    /// The [`Permission`] a capability must carry to delete an existing role
    /// (typically [`Permission::DeleteRoles`]).
    pub delete: Permission,
    /// The [`Permission`] a capability must carry to update the permissions or
    /// tags of an existing role (typically [`Permission::UpdateRoles`]).
    pub update: Permission,
}

/// Defines the permissions required to administer capabilities in this [`RoleMap`].
///
/// When a capability holder attempts to issue or revoke a capability, the
/// `RoleMap` checks that the holder's role includes the corresponding permission
/// listed here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAdminPermissions {
    /// The [`Permission`] a capability must carry to issue (mint) a new capability
    /// (typically [`Permission::AddCapabilities`]).
    pub add: Permission,
    /// The [`Permission`] a capability must carry to revoke an existing capability
    /// or to clean up the revoked-capabilities denylist
    /// (typically [`Permission::RevokeCapabilities`]).
    pub revoke: Permission,
}

/// Options for constraining a newly issued [`Capability`].
///
/// All fields default to `None` (no restriction).  Use [`Default::default()`]
/// to issue an unrestricted capability, or populate individual fields to add
/// constraints.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityIssueOptions {
    /// If set, only the specified address may present the capability.  The Move
    /// runtime rejects any transaction from a different sender with
    /// `ECapabilityIssuedToMismatch`.
    pub issued_to: Option<IotaAddress>,
    /// If set, the capability is not valid before this Unix timestamp
    /// (milliseconds since epoch).  Transactions submitted before this time
    /// are rejected with `ECapabilityTimeConstraintsNotMet`.
    pub valid_from_ms: Option<u64>,
    /// If set, the capability expires after this Unix timestamp (milliseconds
    /// since epoch).  Transactions submitted after this time are rejected with
    /// `ECapabilityTimeConstraintsNotMet`.
    pub valid_until_ms: Option<u64>,
}

/// An allowlist of record tag names that may be attached to a [`Role`].
///
/// When a role carries a `RoleTags` value, capability holders for that role may
/// only add or interact with records whose tag is contained in [`tags`](RoleTags::tags).
/// This maps to the Move `record_tags::RoleTags` type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RoleTags {
    /// The set of tag names this role is allowed to use.  Every tag listed here
    /// must be registered in the trail's tag registry before the role is created
    /// or updated.
    #[serde(deserialize_with = "deserialize_vec_set")]
    pub tags: HashSet<String>,
}

impl RoleTags {
    /// Creates a new [`RoleTags`] from any iterator of string-like items.
    ///
    /// # Arguments
    ///
    /// * `tags` — an iterator of tag names (e.g., `["finance", "legal"]`).
    pub fn new<I, S>(tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tags: tags.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns `true` if `tag` is present in this allowlist.
    ///
    /// # Arguments
    ///
    /// * `tag` — the record tag name to check.
    pub fn allows(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    pub(crate) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!("{package_id}::record_tags::RoleTags")).expect("invalid TypeTag for RoleTags")
    }

    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let mut tags = self.tags.iter().cloned().collect::<Vec<_>>();
        tags.sort();
        let tags_arg = tx::ptb_pure(ptb, "tags", tags)?;

        Ok(ptb.programmable_move_call(
            package_id,
            ident_str!("record_tags").into(),
            ident_str!("new_role_tags").into(),
            vec![],
            vec![tags_arg],
        ))
    }
}

/// An on-chain capability object deserialized from the Move `capability::Capability` type.
///
/// A capability grants its holder the permissions of the [`Role`] identified by
/// [`role`](Capability::role) on the trail identified by
/// [`target_key`](Capability::target_key).  The `RoleMap` validates all fields
/// of the capability before allowing any operation to proceed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// The unique on-chain object ID of this capability.
    pub id: UID,
    /// The object ID of the audit trail this capability is valid for.
    /// Must match the [`RoleMap::target_key`] of the trail being accessed.
    pub target_key: ObjectID,
    /// The name of the role this capability was issued for (e.g., `"Admin"`,
    /// `"RecordAdmin"`).  Determines the set of [`Permission`]s the holder may
    /// exercise.
    pub role: String,
    /// Optional address binding.  When set, only the specified address may
    /// present this capability; any other sender is rejected.
    pub issued_to: Option<IotaAddress>,
    /// Optional start of the validity window (Unix milliseconds).  The
    /// capability is rejected before this timestamp.
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_from: Option<u64>,
    /// Optional end of the validity window (Unix milliseconds).  The capability
    /// is rejected after this timestamp.
    #[serde(deserialize_with = "deserialize_option_number_from_string")]
    pub valid_until: Option<u64>,
}

impl Capability {
    /// Returns the Move `TypeTag` for `capability::Capability` in the given package.
    pub(crate) fn type_tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(format!("{package_id}::capability::Capability").as_str()).expect("failed to create type tag")
    }

    /// Returns `true` if this capability targets the given trail and its role is
    /// contained in `valid_roles`.
    ///
    /// # Arguments
    ///
    /// * `trail_id` — the object ID of the trail to match against.
    /// * `valid_roles` — the set of role names considered acceptable.
    pub(crate) fn matches_target_and_role(&self, trail_id: ObjectID, valid_roles: &HashSet<String>) -> bool {
        self.target_key == trail_id && valid_roles.contains(&self.role)
    }
}

impl MoveType for Capability {
    fn move_type(package: ObjectID) -> TypeTag {
        Self::type_tag(package)
    }
}
