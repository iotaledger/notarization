// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashMap};

use iota_interaction::move_types::annotated_value::MoveValue;
use iota_interaction::rpc_types::{IotaData as _, IotaMoveValue, IotaObjectDataOptions};
use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::types::collection_types::{LinkedTable, LinkedTableNode};
use iota_interaction::types::dynamic_field::{DynamicFieldName, Field};
use iota_interaction::{IotaClientTrait, IotaKeySignature, OptionalSync};
use product_common::core_client::{CoreClient, CoreClientReadOnly};
use product_common::transaction::transaction_builder::TransactionBuilder;
use secret_storage::Signer;
use serde::de::DeserializeOwned;

use crate::core::trail::{AuditTrailFull, AuditTrailReadOnly};
use crate::core::types::{Data, PaginatedRecord, Record};
use crate::error::Error;

mod operations;
mod transactions;

pub use transactions::{AddRecord, DeleteRecord};

use self::operations::RecordsOps;

const MAX_LIST_PAGE_LIMIT: usize = 1_000;

#[derive(Debug, Clone)]
pub struct TrailRecords<'a, C, D = Data> {
    pub(crate) client: &'a C,
    pub(crate) trail_id: ObjectID,
    pub(crate) _phantom: std::marker::PhantomData<D>,
}

impl<'a, C, D> TrailRecords<'a, C, D> {
    pub(crate) fn new(client: &'a C, trail_id: ObjectID) -> Self {
        Self {
            client,
            trail_id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn get(&self, sequence_number: u64) -> Result<Record<D>, Error>
    where
        C: AuditTrailReadOnly,
        D: DeserializeOwned,
    {
        let tx = RecordsOps::get_record(self.client, self.trail_id, sequence_number).await?;
        self.client.execute_read_only_transaction(tx).await
    }

    pub fn add<S>(&self, data: D, metadata: Option<String>) -> TransactionBuilder<AddRecord>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
        D: Into<Data>,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(AddRecord::new(self.trail_id, owner, data.into(), metadata))
    }

    pub fn delete<S>(&self, sequence_number: u64) -> TransactionBuilder<DeleteRecord>
    where
        C: AuditTrailFull + CoreClient<S>,
        S: Signer<IotaKeySignature> + OptionalSync,
    {
        let owner = self.client.sender_address();
        TransactionBuilder::new(DeleteRecord::new(self.trail_id, owner, sequence_number))
    }

    pub async fn correct(&self, _replaces: Vec<u64>, _data: D, _metadata: Option<String>) -> Result<(), Error>
    where
        C: AuditTrailFull,
    {
        Err(Error::NotImplemented("TrailRecords::correct"))
    }

    pub async fn record_count(&self) -> Result<u64, Error>
    where
        C: AuditTrailReadOnly,
    {
        let tx = RecordsOps::record_count(self.client, self.trail_id).await?;
        self.client.execute_read_only_transaction(tx).await
    }

    /// List all records into a [`HashMap`].
    ///
    /// This traverses the full on-chain linked table and can be expensive for large trails.
    /// For paginated access, use [`list_page`](Self::list_page).
    pub async fn list(&self) -> Result<HashMap<u64, Record<D>>, Error>
    where
        C: AuditTrailReadOnly,
        D: DeserializeOwned,
    {
        let records_table = self.load_records_table().await?;
        list_linked_table::<_, Record<D>>(self.client, &records_table, None).await
    }

    /// List all records with a hard cap to protect against expensive traversals.
    pub async fn list_with_limit(&self, max_entries: usize) -> Result<HashMap<u64, Record<D>>, Error>
    where
        C: AuditTrailReadOnly,
        D: DeserializeOwned,
    {
        let records_table = self.load_records_table().await?;
        list_linked_table::<_, Record<D>>(self.client, &records_table, Some(max_entries)).await
    }

    /// List one page of linked-table records starting from `cursor`.
    ///
    /// Pass `None` for the first page; use `next_cursor` for subsequent pages.
    pub async fn list_page(&self, cursor: Option<u64>, limit: usize) -> Result<PaginatedRecord<D>, Error>
    where
        C: AuditTrailReadOnly,
        D: DeserializeOwned,
    {
        if limit > MAX_LIST_PAGE_LIMIT {
            return Err(Error::InvalidArgument(format!(
                "page limit {limit} exceeds max supported page size {MAX_LIST_PAGE_LIMIT}"
            )));
        }

        let records_table = self.load_records_table().await?;
        let (records, next_cursor) =
            list_linked_table_page::<_, Record<D>>(self.client, &records_table, cursor, limit).await?;

        Ok(PaginatedRecord {
            has_next_page: next_cursor.is_some(),
            next_cursor,
            records,
        })
    }

    async fn load_records_table(&self) -> Result<LinkedTable<u64>, Error>
    where
        C: AuditTrailReadOnly,
    {
        crate::core::operations::get_audit_trail(self.trail_id, self.client)
            .await
            .map(|on_chain_trail| on_chain_trail.records)
    }
}

async fn list_linked_table_page<C, V>(
    client: &C,
    table: &LinkedTable<u64>,
    start_key: Option<u64>,
    limit: usize,
) -> Result<(BTreeMap<u64, V>, Option<u64>), Error>
where
    C: CoreClientReadOnly + OptionalSync,
    V: DeserializeOwned,
{
    if limit == 0 {
        return Ok((BTreeMap::new(), start_key.or(table.head)));
    }

    let mut cursor = start_key.or(table.head);
    let mut items = BTreeMap::new();

    for _ in 0..limit {
        let Some(key) = cursor else { break };

        if items.contains_key(&key) {
            return Err(Error::UnexpectedApiResponse(format!(
                "cycle detected while traversing linked-table {table_id}; repeated key {key}",
                table_id = table.id
            )));
        }

        let node = fetch_linked_table_node::<_, V>(client, table.id, key).await?;

        cursor = node.next;
        items.insert(key, node.value);
    }

    Ok((items, cursor))
}

async fn list_linked_table<C, V>(
    client: &C,
    table: &LinkedTable<u64>,
    max_entries: Option<usize>,
) -> Result<HashMap<u64, V>, Error>
where
    C: CoreClientReadOnly + OptionalSync,
    V: DeserializeOwned,
{
    let expected = table.size as usize;
    let cap = max_entries.unwrap_or(expected);

    if expected > cap {
        return Err(Error::InvalidArgument(format!(
            "linked-table size {expected} exceeds max_entries {cap}"
        )));
    }

    let (entries, next_key) = list_linked_table_page(client, table, None, expected).await?;

    if entries.len() != expected {
        return Err(Error::UnexpectedApiResponse(format!(
            "linked-table traversal mismatch; expected {expected} entries, got {}",
            entries.len()
        )));
    }

    if next_key.is_some() {
        return Err(Error::UnexpectedApiResponse(format!(
            "linked-table traversal has extra entries beyond declared size {expected}"
        )));
    }

    Ok(entries.into_iter().collect())
}

async fn fetch_linked_table_node<C, V>(
    client: &C,
    table_id: ObjectID,
    key: u64,
) -> Result<LinkedTableNode<u64, V>, Error>
where
    C: CoreClientReadOnly + OptionalSync,
    V: DeserializeOwned,
{
    let name = DynamicFieldName {
        type_: TypeTag::U64,
        value: IotaMoveValue::from(MoveValue::U64(key)).to_json_value(),
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

    let field: Field<u64, LinkedTableNode<u64, V>> = data
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
