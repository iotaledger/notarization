// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use iota_interaction::rpc_types::{IotaData as _, IotaObjectDataOptions};
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::collection_types::LinkedTableNode;
use iota_interaction::types::dynamic_field::{DynamicFieldName, Field};
use iota_interaction::{IotaClientTrait, OptionalSync};
use product_common::core_client::CoreClientReadOnly;
use serde::de::DeserializeOwned;

use crate::error::Error;

pub(crate) async fn fetch_node<C, K, V>(
    client: &C,
    table_id: ObjectID,
    name: DynamicFieldName,
) -> Result<LinkedTableNode<K, V>, Error>
where
    C: CoreClientReadOnly + OptionalSync,
    K: DeserializeOwned,
    V: DeserializeOwned,
{
    let name_display = name.to_string();
    let data = client
        .client_adapter()
        .read_api()
        .get_dynamic_field_object_v2(table_id, name, Some(IotaObjectDataOptions::bcs_lossless()))
        .await
        .map_err(|err| Error::RpcError(err.to_string()))?
        .data
        .ok_or_else(|| {
            Error::UnexpectedApiResponse(format!(
                "dynamic-field object not found for linked-table id {table_id} and name {name_display}"
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
