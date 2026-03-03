// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::str::FromStr;

use iota_interaction::rpc_types::IotaObjectDataOptions;
use iota_interaction::types::base_types::{ObjectID, ObjectRef, STD_OPTION_MODULE_NAME};
use iota_interaction::types::collection_types::{VecMap, VecSet};
use iota_interaction::types::object::Owner;
use iota_interaction::types::programmable_transaction_builder::{
    ProgrammableTransactionBuilder as Ptb, ProgrammableTransactionBuilder,
};
use iota_interaction::types::transaction::{Argument, ObjectArg};
use iota_interaction::types::{
    IOTA_CLOCK_OBJECT_ID, IOTA_CLOCK_OBJECT_SHARED_VERSION, MOVE_STDLIB_PACKAGE_ID, TypeTag,
};
use iota_interaction::{IotaClientTrait, OptionalSync, ident_str};
use product_common::core_client::CoreClientReadOnly;
use serde::{Deserialize, Deserializer, Serialize};

use crate::error::Error;

/// Adds a reference to the on-chain clock to `ptb`'s arguments.
pub(crate) fn get_clock_ref(ptb: &mut Ptb) -> Argument {
    ptb.obj(ObjectArg::SharedObject {
        id: IOTA_CLOCK_OBJECT_ID,
        initial_shared_version: IOTA_CLOCK_OBJECT_SHARED_VERSION,
        mutable: false,
    })
    .expect("network has a singleton clock instantiated")
}

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

/// Get the type tag of an object
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

    let tag = TypeTag::from_str(&type_param_str)
        .map_err(|e| Error::FailedToParseTag(format!("Failed to parse tag '{type_param_str}': {e}")))?;

    Ok(tag)
}

/// Parses the type string to get the generic argument
pub(crate) fn parse_type(full_type: &str) -> Result<String, Error> {
    if let (Some(start), Some(end)) = (full_type.find('<'), full_type.rfind('>')) {
        Ok(full_type[start + 1..end].to_string())
    } else {
        Err(Error::FailedToParseTag(format!(
            "Could not parse type parameter from {full_type}"
        )))
    }
}

pub(crate) async fn get_object_ref_by_id(
    iota_client: &impl CoreClientReadOnly,
    obj: &ObjectID,
) -> Result<ObjectRef, Error> {
    let res = iota_client
        .client_adapter()
        .read_api()
        .get_object_with_options(*obj, IotaObjectDataOptions::new().with_content())
        .await
        .map_err(|err| Error::GenericError(format!("Failed to get object: {err}")))?;

    let Some(data) = res.data else {
        return Err(Error::InvalidArgument("no data found".to_string()));
    };

    Ok(data.object_ref())
}

pub(crate) async fn get_shared_object_arg(
    iota_client: &impl CoreClientReadOnly,
    obj: &ObjectID,
    mutable: bool,
) -> Result<ObjectArg, Error> {
    let res = iota_client
        .client_adapter()
        .read_api()
        .get_object_with_options(*obj, IotaObjectDataOptions::new().with_owner())
        .await
        .map_err(|err| Error::GenericError(format!("Failed to get object: {err}")))?;

    let Some(data) = res.data else {
        return Err(Error::InvalidArgument("no data found".to_string()));
    };

    match data.owner {
        Some(Owner::Shared { initial_shared_version }) => Ok(ObjectArg::SharedObject {
            id: *obj,
            initial_shared_version,
            mutable,
        }),
        _ => Err(Error::InvalidArgument("object is not shared".to_string())),
    }
}

/// Deserialize a [`VecMap`] into a [`HashMap`]
pub(crate) fn deserialize_vec_map<'de, D, K, V>(deserializer: D) -> Result<HashMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Eq + Hash + Debug,
    V: Deserialize<'de> + Debug,
{
    let vec_map = VecMap::<K, V>::deserialize(deserializer)?;
    Ok(vec_map
        .contents
        .into_iter()
        .map(|entry| (entry.key, entry.value))
        .collect())
}

/// Deserialize a [`VecSet`] into a [`HashSet`]
pub(crate) fn deserialize_vec_set<'de, D, T>(deserializer: D) -> Result<HashSet<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Eq + Hash,
{
    let vec_set = VecSet::<T>::deserialize(deserializer)?;
    Ok(vec_set.contents.into_iter().collect())
}

/// Convert an option value into a [`ProgrammableMoveCall`] argument
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
