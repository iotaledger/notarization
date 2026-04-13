// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Internal access-control helpers that build role and capability transactions.
//!
//! These helpers encode Rust-side access inputs into the exact Move call shapes expected by the audit-trail
//! package and apply the lightweight preflight checks that are cheaper to surface before submission.

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::{ObjectArg, ProgrammableTransaction};
use iota_interaction::{OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;

use crate::core::internal::{trail as trail_reader, tx};
use crate::core::types::{CapabilityIssueOptions, Permission, PermissionSet, RoleTags};
use crate::error::Error;

/// Internal namespace for role and capability transaction construction.
///
/// Each helper selects the required authorization permission, prepares
/// Move-compatible arguments, and then
/// delegates to the shared trail transaction builders in [`crate::core::internal::tx`].
pub(super) struct AccessOps;

impl AccessOps {
    /// Builds the `create_role` call.
    ///
    /// `role_tags`, when present, are validated against the trail tag registry
    /// before PTB construction so the
    /// Rust side fails early with `Error::InvalidArgument` instead of relying on a later Move abort.
    pub(super) async fn create_role<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        name: String,
        permissions: PermissionSet,
        role_tags: Option<RoleTags>,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        assert_role_tags_defined(client, trail_id, &role_tags).await?;

        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::AddRoles,
            selected_capability_id,
            "create_role",
            |ptb, _| {
                let role = tx::ptb_pure(ptb, "role", name)?;
                let perms_vec = permissions.to_move_vec(client.package_id(), ptb)?;
                let perms = ptb.programmable_move_call(
                    client.package_id(),
                    ident_str!("permission").into(),
                    ident_str!("from_vec").into(),
                    vec![],
                    vec![perms_vec],
                );
                let role_tags_arg = match role_tags {
                    Some(role_tags) => {
                        let role_tags_arg = role_tags.to_ptb(ptb, client.package_id())?;

                        tx::option_to_move(Some(role_tags_arg), RoleTags::tag(client.package_id()), ptb)
                            .map_err(|e| Error::InvalidArgument(format!("failed to build role_tags option: {e}")))?
                    }
                    None => tx::option_to_move(None, RoleTags::tag(client.package_id()), ptb)
                        .map_err(|e| Error::InvalidArgument(format!("failed to build role_tags option: {e}")))?,
                };
                let clock = tx::get_clock_ref(ptb);

                Ok(vec![role, perms, role_tags_arg, clock])
            },
        )
        .await
    }

    /// Builds the `update_role_permissions` call.
    ///
    /// The same tag-registry precondition as [`AccessOps::create_role`] applies because role-tag data is stored
    /// on-chain as part of the role definition.
    pub(super) async fn update_role<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        name: String,
        permissions: PermissionSet,
        role_tags: Option<RoleTags>,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        assert_role_tags_defined(client, trail_id, &role_tags).await?;

        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::UpdateRoles,
            selected_capability_id,
            "update_role_permissions",
            |ptb, _| {
                let role = tx::ptb_pure(ptb, "role", name)?;
                let perms_vec = permissions.to_move_vec(client.package_id(), ptb)?;

                let perms = ptb.programmable_move_call(
                    client.package_id(),
                    ident_str!("permission").into(),
                    ident_str!("from_vec").into(),
                    vec![],
                    vec![perms_vec],
                );
                let role_tags_arg = match role_tags {
                    Some(role_tags) => {
                        let role_tags_arg = role_tags.to_ptb(ptb, client.package_id())?;
                        tx::option_to_move(Some(role_tags_arg), RoleTags::tag(client.package_id()), ptb)
                            .map_err(|e| Error::InvalidArgument(format!("failed to build role_tags option: {e}")))?
                    }
                    None => tx::option_to_move(None, RoleTags::tag(client.package_id()), ptb)
                        .map_err(|e| Error::InvalidArgument(format!("failed to build role_tags option: {e}")))?,
                };

                let clock = tx::get_clock_ref(ptb);

                Ok(vec![role, perms, role_tags_arg, clock])
            },
        )
        .await
    }

    /// Builds the `delete_role` call.
    ///
    /// The PTB only carries the role name and clock reference. Protection of the initial-admin role remains an
    /// access-control invariant enforced by the Move package.
    pub(super) async fn delete_role<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        name: String,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteRoles,
            selected_capability_id,
            "delete_role",
            |ptb, _| {
                let role = tx::ptb_pure(ptb, "role", name)?;
                let clock = tx::get_clock_ref(ptb);

                Ok(vec![role, clock])
            },
        )
        .await
    }

    /// Builds the `new_capability` call for a role.
    ///
    /// Optional restrictions are serialized exactly as provided. Validation of `issued_to`, `valid_from`, and
    /// `valid_until` semantics remains on-chain.
    pub(super) async fn issue_capability<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        role_name: String,
        options: CapabilityIssueOptions,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::AddCapabilities,
            selected_capability_id,
            "new_capability",
            |ptb, _| {
                let role = tx::ptb_pure(ptb, "role", role_name)?;
                let issued_to = tx::ptb_pure(ptb, "issued_to", options.issued_to)?;
                let valid_from = tx::ptb_pure(ptb, "valid_from", options.valid_from_ms)?;
                let valid_until = tx::ptb_pure(ptb, "valid_until", options.valid_until_ms)?;
                let clock = tx::get_clock_ref(ptb);

                Ok(vec![role, issued_to, valid_from, valid_until, clock])
            },
        )
        .await
    }

    /// Builds the generic `revoke_capability` call.
    ///
    /// `capability_valid_until` is forwarded to the Move layer so the denylist can later be cleaned up without
    /// losing the capability's original expiry boundary.
    pub(super) async fn revoke_capability<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        capability_id: ObjectID,
        capability_valid_until: Option<u64>,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
            selected_capability_id,
            "revoke_capability",
            |ptb, _| {
                let cap = tx::ptb_pure(ptb, "capability_id", capability_id)?;
                let valid_until = tx::ptb_pure(ptb, "capability_valid_until", capability_valid_until)?;
                let clock = tx::get_clock_ref(ptb);

                Ok(vec![cap, valid_until, clock])
            },
        )
        .await
    }

    /// Builds the generic `destroy_capability` call.
    ///
    /// This resolves the capability object reference up front because the Move entry point consumes the owned
    /// capability object rather than only its ID.
    pub(super) async fn destroy_capability<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        capability_id: ObjectID,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let capability_ref = tx::get_object_ref_by_id(client, &capability_id).await?;

        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
            selected_capability_id,
            "destroy_capability",
            |ptb, _| {
                let cap_to_destroy = ptb
                    .obj(ObjectArg::ImmOrOwnedObject(capability_ref))
                    .map_err(|e| Error::InvalidArgument(format!("Failed to create capability argument: {e}")))?;
                let clock = tx::get_clock_ref(ptb);

                Ok(vec![cap_to_destroy, clock])
            },
        )
        .await
    }

    /// Builds the dedicated `destroy_initial_admin_capability` call.
    ///
    /// Initial-admin capability IDs are tracked separately, so they cannot be destroyed through the generic
    /// capability path.
    pub(super) async fn destroy_initial_admin_capability<C>(
        client: &C,
        trail_id: ObjectID,
        capability_id: ObjectID,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let cap_ref = tx::get_object_ref_by_id(client, &capability_id).await?;
        tx::build_trail_transaction_with_cap_ref(
            client,
            trail_id,
            cap_ref,
            "destroy_initial_admin_capability",
            |_, _| Ok(vec![]),
        )
        .await
    }

    /// Builds the dedicated `revoke_initial_admin_capability` call.
    ///
    /// This keeps the same denylist-expiry behavior as [`AccessOps::revoke_capability`] while using the
    /// separate Move entry point reserved for tracked initial-admin IDs.
    pub(super) async fn revoke_initial_admin_capability<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        capability_id: ObjectID,
        capability_valid_until: Option<u64>,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
            selected_capability_id,
            "revoke_initial_admin_capability",
            |ptb, _| {
                let cap = tx::ptb_pure(ptb, "capability_id", capability_id)?;
                let valid_until = tx::ptb_pure(ptb, "capability_valid_until", capability_valid_until)?;
                let clock = tx::get_clock_ref(ptb);

                Ok(vec![cap, valid_until, clock])
            },
        )
        .await
    }

    /// Builds the `cleanup_revoked_capabilities` call.
    ///
    /// Cleanup only prunes denylist entries whose stored expiry has elapsed. It does not change capability
    /// objects and does not revoke any additional IDs.
    pub(super) async fn cleanup_revoked_capabilities<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        selected_capability_id: Option<ObjectID>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
            selected_capability_id,
            "cleanup_revoked_capabilities",
            |ptb, _| {
                let clock = tx::get_clock_ref(ptb);
                Ok(vec![clock])
            },
        )
        .await
    }
}

/// Verifies that every requested role tag already exists in the trail tag registry.
///
/// Roles may only reference tags that are defined on the trail itself so later record-tag checks
/// stay consistent with the registry stored on-chain.
async fn assert_role_tags_defined<C>(client: &C, trail_id: ObjectID, role_tags: &Option<RoleTags>) -> Result<(), Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let Some(role_tags) = role_tags else {
        return Ok(());
    };

    let trail = trail_reader::get_audit_trail(trail_id, client).await?;
    let undefined_tags = role_tags
        .tags
        .iter()
        .filter(|tag| !trail.tags.contains_key(tag))
        .cloned()
        .collect::<Vec<_>>();

    if undefined_tags.is_empty() {
        Ok(())
    } else {
        Err(Error::InvalidArgument(format!(
            "role tags {:?} are not defined for trail {trail_id}",
            undefined_tags
        )))
    }
}
