// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::OptionalSync;
use iota_interaction::types::base_types::{IotaAddress, ObjectID, ObjectRef};
use iota_interaction::types::transaction::ProgrammableTransaction;
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::{Capability, Permission};
use crate::core::{operations, utils};
use crate::error::Error;

pub(super) struct TrailOps;

impl TrailOps {
    pub(super) async fn migrate<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(client, trail_id, owner, Permission::Migrate, "migrate", |ptb, _| {
            let clock = utils::get_clock_ref(ptb);
            Ok(vec![clock])
        })
        .await
    }

    pub(super) async fn update_metadata<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        metadata: Option<String>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::UpdateMetadata,
            "update_metadata",
            |ptb, _| {
                let metadata_arg = utils::ptb_pure(ptb, "new_metadata", metadata)?;
                let clock = utils::get_clock_ref(ptb);
                Ok(vec![metadata_arg, clock])
            },
        )
        .await
    }

    pub(super) async fn delete_audit_trail<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        operations::build_trail_transaction(
            client,
            trail_id,
            owner,
            Permission::DeleteAuditTrail,
            "delete_audit_trail",
            |ptb, _| {
                let clock = utils::get_clock_ref(ptb);
                Ok(vec![clock])
            },
        )
        .await
    }

    pub(super) async fn add_record_tag<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        tag: String,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        build_admin_trail_transaction(client, trail_id, owner, "add_available_record_tag", |ptb| {
            let tag_arg = utils::ptb_pure(ptb, "tag", tag)?;
            let clock = utils::get_clock_ref(ptb);
            Ok(vec![tag_arg, clock])
        })
        .await
    }

    pub(super) async fn remove_record_tag<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        tag: String,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        build_admin_trail_transaction(client, trail_id, owner, "remove_available_record_tag", |ptb| {
            let tag_arg = utils::ptb_pure(ptb, "tag", tag)?;
            let clock = utils::get_clock_ref(ptb);
            Ok(vec![tag_arg, clock])
        })
        .await
    }

    pub(super) async fn set_record_tags<C>(
        client: &C,
        trail_id: ObjectID,
        owner: IotaAddress,
        tags: Vec<String>,
    ) -> Result<ProgrammableTransaction, Error>
    where
        C: CoreClientReadOnly + OptionalSync,
    {
        build_admin_trail_transaction(client, trail_id, owner, "set_available_record_tags", |ptb| {
            let tags_arg = utils::ptb_pure(ptb, "tags", tags)?;
            let clock = utils::get_clock_ref(ptb);
            Ok(vec![tags_arg, clock])
        })
        .await
    }
}

async fn build_admin_trail_transaction<C, F>(
    client: &C,
    trail_id: ObjectID,
    owner: IotaAddress,
    method: impl AsRef<str>,
    additional_args: F,
) -> Result<ProgrammableTransaction, Error>
where
    F: FnOnce(
        &mut iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder,
    ) -> Result<Vec<iota_interaction::types::transaction::Argument>, Error>,
    C: CoreClientReadOnly + OptionalSync,
{
    let trail = operations::get_audit_trail(trail_id, client).await?;
    let admin_cap_ref = find_admin_cap(client, owner, trail_id, &trail.roles.initial_admin_role_name).await?;

    operations::build_trail_transaction_with_cap_ref(client, trail_id, admin_cap_ref, method, |ptb, _| {
        additional_args(ptb)
    })
    .await
}

async fn find_admin_cap<C>(
    client: &C,
    owner: IotaAddress,
    trail_id: ObjectID,
    admin_role_name: &str,
) -> Result<ObjectRef, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let cap: Capability = client
        .find_object_for_address(owner, |cap: &Capability| {
            cap.target_key == trail_id && cap.role == admin_role_name
        })
        .await
        .map_err(|e| Error::RpcError(e.to_string()))?
        .ok_or_else(|| {
            Error::InvalidArgument(format!(
                "no Admin capability found for owner {owner} and trail {trail_id}"
            ))
        })?;

    let object_id = *cap.id.object_id();
    utils::get_object_ref_by_id(client, &object_id).await
}
