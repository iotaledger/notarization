// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Internal access-control helpers that build role and capability transactions.

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::{ObjectArg, ProgrammableTransaction};
use iota_interaction::{OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;

use crate::core::internal::{trail as trail_reader, tx};
use crate::core::types::{CapabilityIssueOptions, Permission, PermissionSet, RoleTags};
use crate::error::Error;

/// Internal namespace for role and capability transaction construction.
pub(super) struct AccessOps;

impl AccessOps {
    pub(super) async fn create_role<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        name: String,
        permissions: PermissionSet,
        role_tags: Option<RoleTags>,
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

    pub(super) async fn update_role<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        name: String,
        permissions: PermissionSet,
        role_tags: Option<RoleTags>,
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

    pub(super) async fn delete_role<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        name: String,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteRoles,
            "delete_role",
            |ptb, _| {
                let role = tx::ptb_pure(ptb, "role", name)?;
                let clock = tx::get_clock_ref(ptb);

                Ok(vec![role, clock])
            },
        )
        .await
    }

    pub(super) async fn issue_capability<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        role_name: String,
        options: CapabilityIssueOptions,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::AddCapabilities,
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

    pub(super) async fn revoke_capability<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        capability_id: ObjectID,
        capability_valid_until: Option<u64>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
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

    pub(super) async fn destroy_capability<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        capability_id: ObjectID,
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

    pub(super) async fn revoke_initial_admin_capability<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        capability_id: ObjectID,
        capability_valid_until: Option<u64>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
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

    pub(super) async fn cleanup_revoked_capabilities<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        tx::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
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
