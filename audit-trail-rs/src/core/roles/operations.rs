// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::transaction::{Argument, Command, ObjectArg, ProgrammableTransaction};
use iota_interaction::{OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::{CapabilityIssueOptions, Permission, PermissionSet};
use crate::core::{operations, utils};
use crate::error::Error;

pub(super) struct RolesOps;

impl RolesOps {
    pub(super) async fn create_role<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        name: String,
        permissions: PermissionSet,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::AddRoles,
            "create_role",
            |ptb, _| {
                let role = utils::ptb_pure(ptb, "role", name)?;
                let perms_vec = permissions.to_move_vec(client.package_id(), ptb)?;
                let perms = ptb.programmable_move_call(
                    client.package_id(),
                    ident_str!("permission").into(),
                    ident_str!("from_vec").into(),
                    vec![],
                    vec![perms_vec],
                );
                let clock = utils::get_clock_ref(ptb);

                Ok(vec![role, perms, clock])
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
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::UpdateRoles,
            "update_role_permissions",
            |ptb, _| {
                let role = utils::ptb_pure(ptb, "role", name)?;
                let perms_vec = permissions.to_move_vec(client.package_id(), ptb)?;

                let perms = ptb.programmable_move_call(
                    client.package_id(),
                    ident_str!("permission").into(),
                    ident_str!("from_vec").into(),
                    vec![],
                    vec![perms_vec],
                );

                let clock = utils::get_clock_ref(ptb);

                Ok(vec![role, perms, clock])
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
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteRoles,
            "delete_role",
            |ptb, _| {
                let role = utils::ptb_pure(ptb, "role", name)?;
                let clock = utils::get_clock_ref(ptb);

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
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::AddCapabilities,
            "new_capability",
            |ptb, _| {
                let role = utils::ptb_pure(ptb, "role", role_name)?;
                let issued_to = utils::ptb_pure(ptb, "issued_to", options.issued_to)?;
                let valid_from = utils::ptb_pure(ptb, "valid_from", options.valid_from_ms)?;
                let valid_until = utils::ptb_pure(ptb, "valid_until", options.valid_until_ms)?;
                let clock = utils::get_clock_ref(ptb);

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
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
            "revoke_capability",
            |ptb, _| {
                let cap = utils::ptb_pure(ptb, "capability_id", capability_id)?;
                let clock = utils::get_clock_ref(ptb);

                Ok(vec![cap, clock])
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
        let capability_ref = utils::get_object_ref_by_id(client, &capability_id).await?;

        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
            "destroy_capability",
            |ptb, _| {
                let cap_to_destroy = ptb
                    .obj(ObjectArg::ImmOrOwnedObject(capability_ref))
                    .map_err(|e| Error::InvalidArgument(format!("Failed to create capability argument: {e}")))?;
                let clock = utils::get_clock_ref(ptb);

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
        let cap_ref = utils::get_object_ref_by_id(client, &capability_id).await?;
        operations::build_trail_transaction_with_cap_ref(
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
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::RevokeCapabilities,
            "revoke_initial_admin_capability",
            |ptb, _| {
                let cap = utils::ptb_pure(ptb, "capability_id", capability_id)?;
                let clock = utils::get_clock_ref(ptb);

                Ok(vec![cap, clock])
            },
        )
        .await
    }
}
