// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::str::FromStr;

use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID, ObjectRef};
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder;
use iota_interaction::types::transaction::{Argument, ObjectArg, ProgrammableTransaction};
use iota_interaction::{OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;

use crate::core::types::Capability;
use crate::core::utils;
use crate::error::Error;

pub(crate) async fn build_trail_transaction_for_owner<C, F>(
    client: &C,
    trail_id: ObjectID,
    owner: IotaAddress,
    method: impl AsRef<str>,
    additional_args: F,
) -> Result<ProgrammableTransaction, Error>
where
    F: FnOnce(&mut ProgrammableTransactionBuilder, &TypeTag) -> Result<Vec<Argument>, Error>,
    C: CoreClientReadOnly + OptionalSync,
{
    let cap_ref = get_capability_ref(client, owner, trail_id).await?;
    build_trail_transaction_with_cap_ref(client, trail_id, cap_ref, method, additional_args).await
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

    let function = iota_interaction::types::Identifier::from_str(method.as_ref())
        .map_err(|e| Error::InvalidArgument(format!("Invalid method name '{}': {e}", method.as_ref())))?;

    ptb.programmable_move_call(client.package_id(), ident_str!("main").into(), function, tag, args);

    Ok(ptb.finish())
}

pub(crate) async fn get_capability_ref<C>(
    client: &C,
    owner: IotaAddress,
    trail_id: ObjectID,
) -> Result<ObjectRef, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let cap: Capability = client
        .find_object_for_address(owner, |cap: &Capability| cap.target_key == trail_id)
        .await
        .map_err(|e| Error::RpcError(e.to_string()))?
        .ok_or_else(|| Error::InvalidArgument(format!("no capability found for owner {owner} and trail {trail_id}")))?;

    let object_id = *cap.id.object_id();
    utils::get_object_ref_by_id(client, &object_id).await
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
