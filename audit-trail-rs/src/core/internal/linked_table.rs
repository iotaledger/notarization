// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

use iota_interaction::rpc_types::{IotaData as _, IotaObjectDataOptions};
use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::collection_types::{LinkedTable, LinkedTableNode};
use iota_interaction::types::dynamic_field::{DynamicFieldName, Field};
use iota_interaction::{IotaClientTrait, OptionalSync};
use product_common::core_client::CoreClientReadOnly;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::Error;

pub(crate) async fn collect_keys<C, K, V>(
    client: &C,
    table: &LinkedTable<K>,
    key_type: TypeTag,
) -> Result<HashSet<K>, Error>
where
    C: CoreClientReadOnly + OptionalSync,
    K: Clone + DeserializeOwned + Display + Eq + Hash + Serialize,
    V: DeserializeOwned,
{
    let expected = table.size as usize;
    let mut cursor = table.head.clone();
    let mut keys = HashSet::with_capacity(expected);

    while let Some(key) = cursor {
        if !keys.insert(key.clone()) {
            return Err(Error::UnexpectedApiResponse(format!(
                "cycle detected while traversing linked-table {table_id}; repeated key {key}",
                table_id = table.id
            )));
        }

        let node = fetch_node::<_, K, V>(client, table.id, &key, key_type.clone()).await?;
        cursor = node.next;
    }

    if keys.len() != expected {
        return Err(Error::UnexpectedApiResponse(format!(
            "linked-table traversal mismatch; expected {expected} entries, got {}",
            keys.len()
        )));
    }

    Ok(keys)
}

pub(crate) async fn fetch_node<C, K, V>(
    client: &C,
    table_id: ObjectID,
    key: &K,
    key_type: TypeTag,
) -> Result<LinkedTableNode<K, V>, Error>
where
    C: CoreClientReadOnly + OptionalSync,
    K: Clone + DeserializeOwned + Display + Serialize,
    V: DeserializeOwned,
{
    let name = DynamicFieldName {
        type_: key_type,
        value: serde_json::to_value(key).map_err(|err| {
            Error::UnexpectedApiResponse(format!(
                "failed to encode linked-table dynamic-field key {key} for table {table_id}; {err}"
            ))
        })?,
    };

    let data = client
        .client_adapter()
        .read_api()
        .get_dynamic_field_object_v2(table_id, name, Some(IotaObjectDataOptions::bcs_lossless()))
        .await
        .map_err(|err| Error::RpcError(err.to_string()))?
        .data
        .ok_or_else(|| {
            Error::UnexpectedApiResponse(format!(
                "dynamic-field object not found for linked-table id {table_id} and key {key}"
            ))
        })?;

    let field: Field<K, LinkedTableNode<K, V>> = data
        .bcs
        .ok_or_else(|| {
            Error::UnexpectedApiResponse(format!(
                "linked-table node {} missing bcs object content",
                data.object_id
            ))
        })?
        .try_into_move()
        .ok_or_else(|| {
            Error::UnexpectedApiResponse(format!(
                "linked-table node {} bcs content is not a move object",
                data.object_id
            ))
        })?
        .deserialize()
        .map_err(|err| {
            Error::UnexpectedApiResponse(format!("failed to decode linked-table node {}; {err}", data.object_id))
        })?;

    Ok(field.value)
}
