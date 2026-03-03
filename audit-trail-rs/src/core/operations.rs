// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::str::FromStr;

use iota_interaction::rpc_types::{
    IotaData as _, IotaObjectDataOptions, IotaTransactionBlockEffects, IotaTransactionBlockEvents,
};
use iota_interaction::types::base_types::{IotaAddress, ObjectID, ObjectRef};
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::{Argument, ObjectArg, ProgrammableTransaction};
use iota_interaction::types::{Identifier, TypeTag};
use iota_interaction::{IotaClientTrait, OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::{Capability, OnChainAuditTrail, Permission};
use crate::core::utils;
use crate::error::Error;

pub async fn get_audit_trail<C>(trail_id: ObjectID, client: &C) -> Result<OnChainAuditTrail, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let data = client
        .client_adapter()
        .read_api()
        .get_object_with_options(trail_id, IotaObjectDataOptions::bcs_lossless())
        .await
        .map_err(|e| Error::UnexpectedApiResponse(format!("failed to fetch trail {} object; {e}", trail_id)))?
        .data
        .ok_or_else(|| Error::UnexpectedApiResponse(format!("trail {} data not found", trail_id)))?;

    data.bcs
        .ok_or_else(|| Error::UnexpectedApiResponse(format!("trail {} missing bcs object content", trail_id)))?
        .try_into_move()
        .ok_or_else(|| Error::UnexpectedApiResponse(format!("trail {} bcs content is not a move object", trail_id)))?
        .deserialize()
        .map_err(|e| Error::UnexpectedApiResponse(format!("failed to decode trail {} bcs data; {e}", trail_id)))
}

/// Builds a trail transaction by auto-discovering the right capability for the
/// given owner and required permission via the trail's on-chain RoleMap.
pub(crate) async fn build_trail_transaction<C, F>(
    client: &C,
    trail_id: ObjectID,
    owner: IotaAddress,
    permission: Permission,
    method: impl AsRef<str>,
    additional_args: F,
) -> Result<ProgrammableTransaction, Error>
where
    F: FnOnce(&mut ProgrammableTransactionBuilder, &TypeTag) -> Result<Vec<Argument>, Error>,
    C: CoreClientReadOnly + OptionalSync,
{
    let trail = get_audit_trail(trail_id, client).await?;

    let cap_ref = find_capable_cap(client, owner, trail_id, &trail, permission).await?;
    build_trail_transaction_with_cap_ref(client, trail_id, cap_ref, method, additional_args).await
}

/// Finds a capability owned by `owner` whose role has the required permission
/// according to the trail's RoleMap.
async fn find_capable_cap<C>(
    client: &C,
    owner: IotaAddress,
    trail_id: ObjectID,
    trail: &OnChainAuditTrail,
    permission: Permission,
) -> Result<ObjectRef, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let valid_roles: HashSet<&String> = trail
        .roles
        .roles
        .iter()
        .filter(|(_, perms)| perms.contains(&permission))
        .map(|(name, _)| name)
        .collect();

    let cap: Capability = client
        .find_object_for_address(owner, |cap: &Capability| {
            cap.target_key == trail_id && valid_roles.contains(&cap.role)
        })
        .await
        .map_err(|e| Error::RpcError(e.to_string()))?
        .ok_or_else(|| {
            Error::InvalidArgument(format!(
                "no capability with {:?} permission found for owner {owner} and trail {trail_id}",
                permission
            ))
        })?;

    let object_id = *cap.id.object_id();
    utils::get_object_ref_by_id(client, &object_id).await
}

pub(crate) async fn build_trail_transaction_with_cap_ref<C, F>(
    client: &C,
    trail_id: ObjectID,
    cap_ref: ObjectRef,
    method: impl AsRef<str>,
    additional_args: F,
) -> Result<ProgrammableTransaction, Error>
where
    F: FnOnce(&mut ProgrammableTransactionBuilder, &TypeTag) -> Result<Vec<Argument>, Error>,
    C: CoreClientReadOnly + OptionalSync,
{
    let mut ptb = ProgrammableTransactionBuilder::new();

    let type_tag = utils::get_type_tag(client, &trail_id).await?;
    let tag = vec![type_tag.clone()];
    let trail_arg = utils::get_shared_object_arg(client, &trail_id, true).await?;

    let mut args = vec![
        ptb.obj(trail_arg)
            .map_err(|e| Error::InvalidArgument(format!("Failed to create trail argument: {e}")))?,
        ptb.obj(ObjectArg::ImmOrOwnedObject(cap_ref))
            .map_err(|e| Error::InvalidArgument(format!("Failed to create cap argument: {e}")))?,
    ];

    args.extend(additional_args(&mut ptb, &type_tag)?);

    let function = Identifier::from_str(method.as_ref())
        .map_err(|e| Error::InvalidArgument(format!("Invalid method name '{}': {e}", method.as_ref())))?;

    ptb.programmable_move_call(client.package_id(), ident_str!("main").into(), function, tag, args);

    Ok(ptb.finish())
}

pub(crate) async fn build_read_only_transaction<C, F>(
    client: &C,
    trail_id: ObjectID,
    method: impl AsRef<str>,
    additional_args: F,
) -> Result<ProgrammableTransaction, Error>
where
    F: FnOnce(&mut ProgrammableTransactionBuilder) -> Result<Vec<Argument>, Error>,
    C: CoreClientReadOnly + OptionalSync,
{
    let mut ptb = ProgrammableTransactionBuilder::new();

    let tag = vec![utils::get_type_tag(client, &trail_id).await?];
    let trail_arg = utils::get_shared_object_arg(client, &trail_id, false).await?;

    let mut args = vec![
        ptb.obj(trail_arg)
            .map_err(|e| Error::InvalidArgument(format!("Failed to create trail argument: {e}")))?,
    ];

    args.extend(additional_args(&mut ptb)?);

    let function = iota_interaction::types::Identifier::from_str(method.as_ref())
        .map_err(|e| Error::InvalidArgument(format!("Invalid method name '{}': {e}", method.as_ref())))?;

    ptb.programmable_move_call(client.package_id(), ident_str!("main").into(), function, tag, args);

    Ok(ptb.finish())
}
