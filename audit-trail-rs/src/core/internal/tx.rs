// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

//! Shared transaction-building helpers used by the internal audit-trail operations.

use std::str::FromStr;

use iota_interaction::rpc_types::IotaObjectDataOptions;
use iota_interaction::types::base_types::{IotaAddress, ObjectID, ObjectRef, STD_OPTION_MODULE_NAME};
use iota_interaction::types::object::Owner;
use iota_interaction::types::programmable_transaction_builder::{
    ProgrammableTransactionBuilder as Ptb, ProgrammableTransactionBuilder,
};
use iota_interaction::types::transaction::{Argument, ObjectArg, ProgrammableTransaction};
use iota_interaction::types::{
    IOTA_CLOCK_OBJECT_ID, IOTA_CLOCK_OBJECT_SHARED_VERSION, Identifier, MOVE_STDLIB_PACKAGE_ID, TypeTag,
};
use iota_interaction::{IotaClientTrait, OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;
use serde::Serialize;

use super::{capability, trail as trail_reader};
use crate::core::types::Permission;
use crate::error::Error;

/// Returns the canonical immutable clock object argument.
pub(crate) fn get_clock_ref(ptb: &mut Ptb) -> Argument {
    ptb.obj(ObjectArg::SharedObject {
        id: IOTA_CLOCK_OBJECT_ID,
        initial_shared_version: IOTA_CLOCK_OBJECT_SHARED_VERSION,
        mutable: false,
    })
    .expect("network has a singleton clock instantiated")
}

/// Serializes a pure programmable-transaction argument and annotates serialization failures with
/// the logical argument name.
pub(crate) fn ptb_pure<T>(ptb: &mut Ptb, name: &str, value: T) -> Result<Argument, Error>
where
    T: Serialize + core::fmt::Debug,
{
    ptb.pure(&value).map_err(|err| {
        Error::InvalidArgument(format!(
            r"could not serialize pure value {name} with value {value:?}; {err}"
        ))
    })
}

/// Wraps an optional argument into the corresponding Move `std::option::Option<T>` value.
pub(crate) fn option_to_move(
    option: Option<Argument>,
    tag: TypeTag,
    ptb: &mut ProgrammableTransactionBuilder,
) -> Result<Argument, anyhow::Error> {
    let arg = if let Some(t) = option {
        ptb.programmable_move_call(
            MOVE_STDLIB_PACKAGE_ID,
            STD_OPTION_MODULE_NAME.into(),
            ident_str!("some").into(),
            vec![tag],
            vec![t],
        )
    } else {
        ptb.programmable_move_call(
            MOVE_STDLIB_PACKAGE_ID,
            STD_OPTION_MODULE_NAME.into(),
            ident_str!("none").into(),
            vec![tag],
            vec![],
        )
    };

    Ok(arg)
}

/// Builds a writable trail transaction after resolving both the trail object and a matching
/// capability for `owner`.
pub(crate) async fn build_trail_transaction<C, F>(
    client: &C,
    trail_id: ObjectID,
    owner: IotaAddress,
    permission: Permission,
    selected_capability_id: Option<ObjectID>,
    method: impl AsRef<str>,
    additional_args: F,
) -> Result<ProgrammableTransaction, Error>
where
    F: FnOnce(&mut ProgrammableTransactionBuilder, &TypeTag) -> Result<Vec<Argument>, Error>,
    C: CoreClientReadOnly + OptionalSync,
{
    let cap_ref = if let Some(capability_id) = selected_capability_id {
        get_object_ref_by_id(client, &capability_id).await?
    } else {
        let trail = trail_reader::get_audit_trail(trail_id, client).await?;
        capability::find_capable_cap(client, owner, trail_id, &trail, permission).await?
    };
    build_trail_transaction_with_cap_ref(client, trail_id, cap_ref, method, additional_args).await
}

/// Builds a writable trail transaction when the caller already has the capability object
/// reference.
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

    let type_tag = get_type_tag(client, &trail_id).await?;
    let tag = vec![type_tag.clone()];
    let trail_arg = get_shared_object_arg(client, &trail_id, true).await?;

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

/// Builds a read-only trail transaction that borrows the shared trail object immutably.
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

    let tag = vec![get_type_tag(client, &trail_id).await?];
    let trail_arg = get_shared_object_arg(client, &trail_id, false).await?;

    let mut args = vec![
        ptb.obj(trail_arg)
            .map_err(|e| Error::InvalidArgument(format!("Failed to create trail argument: {e}")))?,
    ];

    args.extend(additional_args(&mut ptb)?);

    let function = Identifier::from_str(method.as_ref())
        .map_err(|e| Error::InvalidArgument(format!("Invalid method name '{}': {e}", method.as_ref())))?;

    ptb.programmable_move_call(client.package_id(), ident_str!("main").into(), function, tag, args);

    Ok(ptb.finish())
}

/// Extracts the generic record payload type from the on-chain trail object type.
///
/// Audit-trail Move entry points are generic over the record payload type, so transaction builders
/// need this type tag to invoke the correct specialization.
pub(crate) async fn get_type_tag<C>(client: &C, object_id: &ObjectID) -> Result<TypeTag, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let object_response = client
        .client_adapter()
        .read_api()
        .get_object_with_options(*object_id, IotaObjectDataOptions::new().with_type())
        .await
        .map_err(|err| Error::FailedToParseTag(format!("Failed to get object: {err}")))?;

    let object_data = object_response
        .data
        .ok_or_else(|| Error::FailedToParseTag(format!("Object {object_id} not found")))?;

    let full_type_str = object_data
        .object_type()
        .map_err(|e| Error::FailedToParseTag(format!("Failed to get object type: {e}")))?
        .to_string();

    let type_param_str = parse_type(&full_type_str)?;

    TypeTag::from_str(&type_param_str)
        .map_err(|e| Error::FailedToParseTag(format!("Failed to parse tag '{type_param_str}': {e}")))
}

/// Extracts the innermost generic type parameter from a full Move object type string.
fn parse_type(full_type: &str) -> Result<String, Error> {
    if let (Some(start), Some(end)) = (full_type.find('<'), full_type.rfind('>')) {
        Ok(full_type[start + 1..end].to_string())
    } else {
        Err(Error::FailedToParseTag(format!(
            "Could not parse type parameter from {full_type}"
        )))
    }
}

/// Fetches the current object reference for `object_id`.
pub(crate) async fn get_object_ref_by_id(
    client: &impl CoreClientReadOnly,
    object_id: &ObjectID,
) -> Result<ObjectRef, Error> {
    let res = client
        .client_adapter()
        .read_api()
        .get_object_with_options(*object_id, IotaObjectDataOptions::new().with_content())
        .await
        .map_err(|err| Error::GenericError(format!("Failed to get object: {err}")))?;

    let Some(data) = res.data else {
        return Err(Error::InvalidArgument("no data found".to_string()));
    };

    Ok(data.object_ref())
}

/// Resolves a shared object argument for use in a programmable transaction.
///
/// This validates that the fetched object is shared and returns the appropriate mutability flag for
/// the planned call.
pub(crate) async fn get_shared_object_arg(
    client: &impl CoreClientReadOnly,
    object_id: &ObjectID,
    mutable: bool,
) -> Result<ObjectArg, Error> {
    let res = client
        .client_adapter()
        .read_api()
        .get_object_with_options(*object_id, IotaObjectDataOptions::new().with_owner())
        .await
        .map_err(|err| Error::GenericError(format!("Failed to get object: {err}")))?;

    let Some(data) = res.data else {
        return Err(Error::InvalidArgument("no data found".to_string()));
    };

    match data.owner {
        Some(Owner::Shared { initial_shared_version }) => Ok(ObjectArg::SharedObject {
            id: *object_id,
            initial_shared_version,
            mutable,
        }),
        _ => Err(Error::InvalidArgument("object is not shared".to_string())),
    }
}
