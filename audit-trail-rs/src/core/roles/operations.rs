// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::TypeTag;
use iota_interaction::types::transaction::Command;
use iota_interaction::types::transaction::ObjectArg;
use iota_interaction::types::transaction::ProgrammableTransaction;
use iota_interaction::{OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;
use std::str::FromStr;

use crate::core::operations;
use crate::core::types::{Capability, CapabilityIssueOptions, Permission, PermissionSet};
use crate::core::utils;
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
        let admin_cap_ref = Self::get_admin_capability_ref(client, owner, trail_id).await?;
        operations::build_trail_transaction_with_cap_ref(client, trail_id, admin_cap_ref, "create_role", |ptb, _| {
            let role = utils::ptb_pure(ptb, "role", name)?;
            let perms_vec = Self::permissions_to_vec(ptb, client.package_id(), permissions.permissions);
            let perms = ptb.programmable_move_call(
                client.package_id(),
                ident_str!("permission").into(),
                ident_str!("from_vec").into(),
                vec![],
                vec![perms_vec],
            );
            let clock = utils::get_clock_ref(ptb);

            Ok(vec![role, perms, clock])
        })
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
        let admin_cap_ref = Self::get_admin_capability_ref(client, owner, trail_id).await?;
        operations::build_trail_transaction_with_cap_ref(client, trail_id, admin_cap_ref, "update_role_permissions", |ptb, _| {
            let role = utils::ptb_pure(ptb, "role", name)?;
            let perms_vec = Self::permissions_to_vec(ptb, client.package_id(), permissions.permissions);
            let perms = ptb.programmable_move_call(
                client.package_id(),
                ident_str!("permission").into(),
                ident_str!("from_vec").into(),
                vec![],
                vec![perms_vec],
            );
            let clock = utils::get_clock_ref(ptb);

            Ok(vec![role, perms, clock])
        })
        .await
    }

    fn permissions_to_vec(
        ptb: &mut iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder,
        package_id: ObjectID,
        permissions: Vec<Permission>,
    ) -> iota_interaction::types::transaction::Argument {
        let permission_type = TypeTag::from_str(format!("{package_id}::permission::Permission").as_str())
            .expect("invalid TypeTag for Permission");
        let permission_args = permissions
            .into_iter()
            .map(|permission| Self::permission_to_argument(ptb, package_id, permission))
            .collect::<Vec<_>>();
        ptb.command(Command::MakeMoveVec(Some(permission_type.into()), permission_args))
    }

    fn permission_to_argument(
        ptb: &mut iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder,
        package_id: ObjectID,
        permission: Permission,
    ) -> iota_interaction::types::transaction::Argument {
        let function = match permission {
            Permission::DeleteAuditTrail => ident_str!("delete_audit_trail").into(),
            Permission::AddRecord => ident_str!("add_record").into(),
            Permission::DeleteRecord => ident_str!("delete_record").into(),
            Permission::CorrectRecord => ident_str!("correct_record").into(),
            Permission::UpdateLockingConfig => ident_str!("update_locking_config").into(),
            Permission::UpdateLockingConfigForDeleteRecord => ident_str!("update_locking_config_for_delete_record").into(),
            Permission::UpdateLockingConfigForDeleteTrail => ident_str!("update_locking_config_for_delete_trail").into(),
            Permission::AddRoles => ident_str!("add_roles").into(),
            Permission::UpdateRoles => ident_str!("update_roles").into(),
            Permission::DeleteRoles => ident_str!("delete_roles").into(),
            Permission::AddCapabilities => ident_str!("add_capabilities").into(),
            Permission::RevokeCapabilities => ident_str!("revoke_capabilities").into(),
            Permission::UpdateMetadata => ident_str!("update_metadata").into(),
            Permission::DeleteMetadata => ident_str!("delete_metadata").into(),
        };

        ptb.programmable_move_call(package_id, ident_str!("permission").into(), function, vec![], vec![])
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
        let admin_cap_ref = Self::get_admin_capability_ref(client, owner, trail_id).await?;
        operations::build_trail_transaction_with_cap_ref(client, trail_id, admin_cap_ref, "delete_role", |ptb, _| {
            let role = utils::ptb_pure(ptb, "role", name)?;
            let clock = utils::get_clock_ref(ptb);

            Ok(vec![role, clock])
        })
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
        let admin_cap_ref = Self::get_admin_capability_ref(client, owner, trail_id).await?;
        operations::build_trail_transaction_with_cap_ref(client, trail_id, admin_cap_ref, "new_capability", |ptb, _| {
            let role = utils::ptb_pure(ptb, "role", role_name)?;
            let issued_to = utils::ptb_pure(ptb, "issued_to", options.issued_to)?;
            let valid_from = utils::ptb_pure(ptb, "valid_from", options.valid_from_ms)?;
            let valid_until = utils::ptb_pure(ptb, "valid_until", options.valid_until_ms)?;
            let clock = utils::get_clock_ref(ptb);

            Ok(vec![role, issued_to, valid_from, valid_until, clock])
        })
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
        let admin_cap_ref = Self::get_admin_capability_ref(client, owner, trail_id).await?;
        operations::build_trail_transaction_with_cap_ref(client, trail_id, admin_cap_ref, "revoke_capability", |ptb, _| {
            let cap = utils::ptb_pure(ptb, "capability_id", capability_id)?;
            let clock = utils::get_clock_ref(ptb);

            Ok(vec![cap, clock])
        })
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
        let admin_cap_ref = Self::get_admin_capability_ref(client, owner, trail_id).await?;
        let capability_ref = utils::get_object_ref_by_id(client, &capability_id).await?;

        operations::build_trail_transaction_with_cap_ref(client, trail_id, admin_cap_ref, "destroy_capability", |ptb, _| {
            let cap_to_destroy = ptb
                .obj(ObjectArg::ImmOrOwnedObject(capability_ref))
                .map_err(|e| Error::InvalidArgument(format!("Failed to create capability argument: {e}")))?;
            let clock = utils::get_clock_ref(ptb);

            Ok(vec![cap_to_destroy, clock])
        })
        .await
    }

    async fn get_admin_capability_ref<C>(
        client: &C,
        owner: IotaAddress,
        trail_id: ObjectID,
    ) -> Result<iota_interaction::types::base_types::ObjectRef, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        let admin_cap: Capability = client
            .find_object_for_address(owner, |cap: &Capability| {
                cap.target_key == trail_id && cap.role == "Admin"
            })
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?
            .ok_or_else(|| {
                Error::InvalidArgument(format!(
                    "no admin capability found for owner {owner} and trail {trail_id}"
                ))
            })?;

        let object_id = *admin_cap.id.object_id();
        utils::get_object_ref_by_id(client, &object_id).await
    }
}
